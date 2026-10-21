//! Every method-specific representation maps to its supplying authored method.
use neocogi_idl::{GeneratedRust, ModuleSource, SourceMapKind, SourceMapPosition};
use proc_macro2::Span;
use quote::ToTokens;
use std::path::Path;
use syn::visit::Visit;

/// Authored one-based Unicode position, independent of parser or emitted spans.
fn position(source: &str, needle: &str) -> SourceMapPosition {
    let before = &source[..source.find(needle).expect("authored identifier")];
    SourceMapPosition {
        line: before.chars().filter(|value| *value == '\n').count() + 1,
        column: before.rsplit('\n').next().unwrap().chars().count() + 1,
    }
}

/// Queries selected from parsed generated syntax, never from the map under test.
struct MethodPoints<'a> {
    /// Concrete emitted provider trait, including a closed generic specialization.
    owner: &'a str,
    /// Unique supplying method in this test graph.
    method: &'a str,
    /// Representation, syntax kind and actual formatted-source query position.
    points: Vec<(&'static str, SourceMapKind, Span)>,
}
impl MethodPoints<'_> {
    /// Includes every nested body token and closing delimiter, not just the name.
    fn body(&mut self, label: &'static str, method: &syn::ImplItemFn) {
        self.points.push((label, SourceMapKind::Method, method.sig.ident.span()));
        let mut pending = vec![method.block.to_token_stream()];
        while let Some(tokens) = pending.pop() {
            for token in tokens {
                self.points.push((label, SourceMapKind::Method, token.span()));
                if let proc_macro2::TokenTree::Group(group) = token {
                    self.points.push((label, SourceMapKind::Method, group.span_close()));
                    pending.push(group.stream());
                }
            }
        }
    }
}
impl<'ast> Visit<'ast> for MethodPoints<'_> {
    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        if item.ident == self.owner {
            for member in &item.items {
                if let syn::TraitItem::Fn(method) = member {
                    if method.sig.ident == self.method {
                        self.points.push(("trait", SourceMapKind::Method, method.sig.ident.span()));
                    }
                }
            }
        }
        syn::visit::visit_item_trait(self, item);
    }
    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        if item.ident == format!("{}VTable", self.owner) {
            for field in &item.fields {
                if field.ident.as_ref().is_some_and(|name| name == self.method) {
                    self.points.push(("slot", SourceMapKind::Field, field.ident.as_ref().unwrap().span()));
                }
            }
        }
        syn::visit::visit_item_struct(self, item);
    }
    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        if let syn::Type::Path(path) = &*item.self_ty {
            let name = &path.path.segments.last().unwrap().ident;
            for member in &item.items {
                if let syn::ImplItem::Fn(method) = member {
                    if name == &format!("{}Client", self.owner) && method.sig.ident == self.method {
                        self.body("client", method);
                    } else if name == &format!("{}VtblFor", self.owner) && method.sig.ident == format!("{}_thunk", self.method) {
                        self.body("thunk", method);
                    }
                }
            }
        }
        syn::visit::visit_item_impl(self, item);
    }
}

/// Finds the supplying trait independently, then checks all four representations.
fn assert_method(generated: &GeneratedRust, method: &str, authored: &str, module: &str, path: &str) {
    let syntax = syn::parse_file(&generated.source).unwrap();
    let owner = syntax
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Trait(item)
                if item
                    .items
                    .iter()
                    .any(|member| matches!(member, syn::TraitItem::Fn(item) if item.sig.ident == method)) =>
            {
                Some(item.ident.to_string())
            }
            _ => None,
        })
        .expect("one supplying trait");
    let mut query = MethodPoints {
        owner: &owner,
        method,
        points: Vec::new(),
    };
    query.visit_file(&syntax);
    for label in ["trait", "slot", "client", "thunk"] {
        assert!(query.points.iter().any(|(found, _, _)| *found == label), "missing {owner}::{method} {label}");
    }
    let start = position(authored, &format!("{method}("));
    let end = SourceMapPosition {
        line: start.line,
        column: start.column + method.chars().count(),
    };
    for (label, kind, span) in query.points {
        let point = span.start();
        let at = SourceMapPosition {
            line: point.line,
            column: point.column + 1,
        };
        let entry = generated
            .source_map
            .lookup(at)
            .unwrap_or_else(|| panic!("unmapped {owner}::{method} {label} {at:?}"));
        let file = &generated.source_map.sources[entry.original.source];
        assert_eq!((file.module.as_str(), file.path.as_str()), (module, path), "{label} at {at:?}");
        assert_eq!(entry.original.range, neocogi_idl::SourceMapRange { start, end }, "{label} at {at:?}");
        assert_eq!(entry.kind, kind, "{label} at {at:?}");
    }
}

/// Root and derived-local methods retain their exact range; inherited slots use
/// the supplying base, including an imported template rather than its use site.
#[test]
fn raw_slots_and_complete_thunk_bodies_retain_their_authored_method() {
    let templates = "/* café 😀 */ #[version(1)] interface Source<T> {\n    /// Template member documentation.\n    fn template(&self) -> Result<T>;\n}";
    let consumer = "extern origins.templates; use origins.templates::Source;\n#[version(1)] interface Root {\n    fn root(&self) -> Result<u32>;\n}\n#[version(1)] interface Derived: Root + Source<u32> {\n    /// Local member documentation.\n    fn local(&mut self, value: u32) -> Result<()>;\n}";
    for prefix in ["", "\n\n/* shifted */\n"] {
        let template_text = format!("{prefix}{templates}");
        let consumer_text = format!("{prefix}{consumer}");
        let template = neocogi_idl::parse_module_parts(
            Path::new("origins/templates"),
            &[ModuleSource {
                path: Path::new("origins/templates/members.ncidl"),
                text: &template_text,
            }],
        )
        .unwrap();
        let consumer = neocogi_idl::parse_module(Path::new("origins/methods.ncidl"), &consumer_text).unwrap();
        let graph = neocogi_idl::resolve_modules(vec![template, consumer]).unwrap();
        let generated = neocogi_idl::generate_rust_with_source_map(&graph, "origins.methods").unwrap();
        for method in ["root", "local"] {
            assert_method(&generated, method, &consumer_text, "origins.methods", "origins/methods.ncidl");
        }
        assert_method(&generated, "template", &template_text, "origins.templates", "origins/templates/members.ncidl");
        let syntax = syn::parse_file(&generated.source).unwrap();
        let syn::Item::Struct(derived) = syntax
            .items
            .iter()
            .find(|item| matches!(item, syn::Item::Struct(item) if item.ident == "DerivedVTable"))
            .unwrap()
        else {
            unreachable!()
        };
        assert!(
            !derived
                .fields
                .iter()
                .any(|field| field.ident.as_ref().is_some_and(|name| name == "root" || name == "template")),
            "inherited methods keep their supplying tables"
        );
    }
}
