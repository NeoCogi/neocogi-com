//! Maps formatted syntax, rather than token-printing guesses, back to IDL origins.

use std::collections::BTreeMap;
use proc_macro2::Span;
use quote::ToTokens;
use syn::{
    spanned::Spanned,
    visit::{self, Visit},
    Attribute,
};
use crate::{Declaration, DeclarationKind, ResolvedModule, SourceSpan, VariantFields};
use crate::source_map::*;
use super::GenerateError;

/// Human-readable documentation also identifies an origin during syntax traversal.
/// Authored docs precede this generated annotation, so a copied annotation cannot
/// override the actual origin attached to a definition.
pub(super) fn origin_label(span: &SourceSpan) -> String {
    format!(
        "IDL source: `{}:{}:{}` (module `{}`).",
        span.path(),
        span.start().line,
        span.start().column,
        span.module()
    )
}

/// Copies stable parser coordinates without retaining process-local parser spans.
fn original(span: &SourceSpan, map: &mut RustSourceMap) -> SourceMapOrigin {
    let source = match map.sources.iter().position(|file| file.module == span.module() && file.path == span.path()) {
        Some(index) => index,
        None => {
            map.sources.push(SourceMapFile {
                module: span.module().to_owned(),
                path: span.path().to_owned(),
                location: None,
            });
            map.sources.len() - 1
        }
    };
    SourceMapOrigin {
        source,
        range: SourceMapRange {
            start: SourceMapPosition {
                line: span.start().line,
                column: span.start().column,
            },
            end: SourceMapPosition {
                line: span.end().line,
                column: span.end().column,
            },
        },
    }
}

/// Starts a portable map; Cargo later resolves each file using its owning module.
pub(super) fn new_map(module: &ResolvedModule) -> RustSourceMap {
    RustSourceMap {
        schema: 2,
        module: module.source.name.to_string(),
        generated_file: format!("{}.rs", module.source.name.segments().join("_")),
        sources: Vec::new(),
        mappings: Vec::new(),
    }
}

/// Records source-backed imports while leaving synthetic module metadata unmapped.
pub(super) fn map_header(source: &str, module: &ResolvedModule, map: &mut RustSourceMap) -> Result<(), GenerateError> {
    let origins = module
        .source
        .imports
        .iter()
        .map(|import| (origin_label(&import.span), original(&import.span, map)))
        .collect();
    map_syntax(source, 0, origins, None, &mut map.mappings)
}

/// Maps a complete declaration fragment and refines it with nested syntax ranges.
/// The caller supplies the defining identifier separately from a specialization's
/// diagnostic use site. Authored members still refine that declaration fallback.
pub(super) fn map_declaration(
    source: &str,
    offset: usize,
    declaration: &Declaration,
    declaration_origin: &SourceSpan,
    map: &mut RustSourceMap,
) -> Result<(), GenerateError> {
    let origin = original(declaration_origin, map);
    map.mappings.push(SourceMapping {
        kind: SourceMapKind::Declaration,
        name: declaration.name.clone(),
        generated: SourceMapRange {
            start: SourceMapPosition { line: offset + 1, column: 1 },
            end: SourceMapPosition {
                line: offset + source.lines().count() + 1,
                column: 1,
            },
        },
        original: origin.clone(),
    });
    let mut origins = BTreeMap::new();
    let mut add = |span: &SourceSpan| {
        origins.insert(origin_label(span), original(span, map));
    };
    add(declaration_origin);
    match &declaration.kind {
        DeclarationKind::Struct(record) => {
            for field in &record.fields {
                add(&field.span);
            }
        }
        DeclarationKind::Enum(enumeration) => {
            for variant in &enumeration.variants {
                add(&variant.span);
                if let VariantFields::Named(fields) = &variant.fields {
                    for field in fields {
                        add(&field.span);
                    }
                }
            }
        }
        DeclarationKind::BitEnum(enumeration) => {
            for flag in &enumeration.flags {
                add(&flag.span);
            }
        }
        DeclarationKind::Interface(interface) => {
            for method in &interface.methods {
                add(&method.span);
            }
        }
    }
    map_syntax(source, offset, origins, Some(origin), &mut map.mappings)
}

/// Parses the exact final layout so doc comments, attributes and wrapping are accounted for.
fn map_syntax(
    source: &str,
    offset: usize,
    origins: BTreeMap<String, SourceMapOrigin>,
    parent: Option<SourceMapOrigin>,
    mappings: &mut Vec<SourceMapping>,
) -> Result<(), GenerateError> {
    super::syntax::with_syntax_stack(move || {
        let syntax = syn::parse_file(source).map_err(|error| GenerateError::new(format!("cannot map generated Rust: {error}")))?;
        let mut mapper = Mapper {
            offset,
            origins,
            parent,
            scope: String::new(),
            mappings,
        };
        mapper.visit_file(&syntax);
        Ok(())
    })
}

/// Traverses owned syntax with ordinary exclusive access; no shared mutation is needed.
struct Mapper<'a> {
    /// Number of complete generated lines preceding the current formatted fragment.
    offset: usize,
    /// Known annotations for this fragment; arbitrary authored doc text is ignored.
    origins: BTreeMap<String, SourceMapOrigin>,
    /// Closest authored ancestor, inherited by generated helpers and their bodies.
    parent: Option<SourceMapOrigin>,
    /// Enclosing Rust definition used to distinguish repeated member names.
    scope: String,
    /// Result buffer owned by the module generation call.
    mappings: &'a mut Vec<SourceMapping>,
}
impl Mapper<'_> {
    /// Emits one syntax range under a temporary origin/name scope, then restores its parent.
    fn enter(&mut self, attrs: &[Attribute], span: Span, kind: SourceMapKind, name: String, children: impl FnOnce(&mut Self)) {
        let origin = attrs
            .iter()
            .rev()
            .find_map(|attribute| {
                if !attribute.path().is_ident("doc") {
                    return None;
                }
                let syn::Meta::NameValue(value) = &attribute.meta else {
                    return None;
                };
                let syn::Expr::Lit(value) = &value.value else {
                    return None;
                };
                let syn::Lit::Str(value) = &value.lit else {
                    return None;
                };
                self.origins.get(value.value().trim_start()).cloned()
            })
            .or_else(|| self.parent.clone());
        let name = match self.scope.is_empty() {
            true => name,
            false => format!("{}::{name}", self.scope),
        };
        if let Some(original) = &origin {
            let start = span.start();
            let end = span.end();
            self.mappings.push(SourceMapping {
                kind,
                name: name.clone(),
                generated: SourceMapRange {
                    start: SourceMapPosition {
                        line: self.offset + start.line,
                        column: start.column + 1,
                    },
                    end: SourceMapPosition {
                        line: self.offset + end.line,
                        column: end.column + 1,
                    },
                },
                original: original.clone(),
            });
        }
        let previous_origin = std::mem::replace(&mut self.parent, origin);
        let previous_scope = std::mem::replace(&mut self.scope, name);
        children(self);
        self.scope = previous_scope;
        self.parent = previous_origin;
    }
}
impl<'ast> Visit<'ast> for Mapper<'_> {
    /// Covers every ordinary item form emitted by the typed backend.
    fn visit_item(&mut self, item: &'ast syn::Item) {
        let (attrs, kind, name) = match item {
            syn::Item::Struct(value) => (&value.attrs, SourceMapKind::Struct, value.ident.to_string()),
            syn::Item::Enum(value) => (&value.attrs, SourceMapKind::Enum, value.ident.to_string()),
            syn::Item::Union(value) => (&value.attrs, SourceMapKind::Union, value.ident.to_string()),
            syn::Item::Trait(value) => (&value.attrs, SourceMapKind::Trait, value.ident.to_string()),
            syn::Item::Impl(value) => (&value.attrs, SourceMapKind::Implementation, format!("impl {}", value.self_ty.to_token_stream())),
            syn::Item::Fn(value) => (&value.attrs, SourceMapKind::Function, value.sig.ident.to_string()),
            syn::Item::Const(value) => (&value.attrs, SourceMapKind::Constant, value.ident.to_string()),
            syn::Item::Type(value) => (&value.attrs, SourceMapKind::Type, value.ident.to_string()),
            syn::Item::Use(value) => (&value.attrs, SourceMapKind::Import, value.tree.to_token_stream().to_string()),
            _ => {
                visit::visit_item(self, item);
                return;
            }
        };
        self.enter(attrs, item.span(), kind, name, |mapper| visit::visit_item(mapper, item));
    }

    /// Refines methods independently of nearby generated functions with the same name.
    fn visit_trait_item_fn(&mut self, method: &'ast syn::TraitItemFn) {
        self.enter(&method.attrs, method.span(), SourceMapKind::Method, method.sig.ident.to_string(), |mapper| {
            visit::visit_trait_item_fn(mapper, method)
        });
    }

    /// Includes each generated body so a diagnostic inside it resolves to its IDL cause.
    fn visit_impl_item_fn(&mut self, method: &'ast syn::ImplItemFn) {
        self.enter(&method.attrs, method.span(), SourceMapKind::Method, method.sig.ident.to_string(), |mapper| {
            visit::visit_impl_item_fn(mapper, method)
        });
    }

    /// Named fields carry exact authored origins; synthetic/tuple fields inherit their owner.
    fn visit_field(&mut self, field: &'ast syn::Field) {
        let name = field.ident.as_ref().map_or_else(|| "<positional field>".to_owned(), ToString::to_string);
        self.enter(&field.attrs, field.span(), SourceMapKind::Field, name, |mapper| {
            visit::visit_field(mapper, field)
        });
    }

    /// Variant scopes ensure identically named payload fields remain distinguishable.
    fn visit_variant(&mut self, variant: &'ast syn::Variant) {
        self.enter(&variant.attrs, variant.span(), SourceMapKind::Variant, variant.ident.to_string(), |mapper| {
            visit::visit_variant(mapper, variant)
        });
    }

    /// Scalar enum/flag members are ordinary associated constants with their own IDL origins.
    fn visit_impl_item_const(&mut self, item: &'ast syn::ImplItemConst) {
        self.enter(&item.attrs, item.span(), SourceMapKind::Constant, item.ident.to_string(), |mapper| {
            visit::visit_impl_item_const(mapper, item)
        });
    }
}
