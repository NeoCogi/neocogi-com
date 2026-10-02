//! Generates forms from real COM catalog calls using IDL-published raw identifiers.
use neocogi_com::*;
use neocogi_ui_core_contracts::{IWidgetAbi, IWidgetHostAbi};
use neocogi_ui_document::{CustomWidgetRegistry, Document, Node, design::DesignWidgetManifest};
use neocogi_ui_document::contracts::{IDesignWidgetCatalog, IDesignWidgetCatalogAbi};
use path_contracts::r#type::CounterAbi;

/// Exposes one owned descriptor through generated COM method thunks.
#[com_object(interfaces(neocogi_ui_document::contracts::IDesignWidgetCatalog))]
struct Catalog {
    /// JSON bytes can deliberately bypass producer validation in negative tests.
    bytes: Vec<u8>,
}
impl IDesignWidgetCatalog for Catalog {
    /// This test provider offers construction/metadata only, without a settings editor.
    fn create_editor(&self, _: u32, _: neocogi_com::ComRef<'_, neocogi_ui::UiFactoryAbi>, _: neocogi_com::NcCopyFromBufferOffset<'_, u8>) -> Result<neocogi_com::ComBox<neocogi_ui_document::contracts::IDesignSettingsEditorAbi>, neocogi_com::NcStatus> {
        Err(neocogi_com::NC_E_NOINTERFACE)
    }

    /// One description is sufficient to exercise registry admission and form export.
    fn widget_count(&self) -> Result<u32, NcStatus> {
        Ok(1)
    }

    /// Supports the catalog's length-query then bounded-copy protocol.
    fn describe(&self, index: u32, mut output: NcOutputWriter<'_, u8>) -> Result<u32, NcStatus> {
        if index != 0 {
            return Err(NC_E_INVALID_ARG);
        }
        if output.capacity() >= self.bytes.len() {
            output.extend_from_slice(&self.bytes)?;
        }
        Ok(self.bytes.len() as u32)
    }

    /// Source generation must never construct a widget or require a running UI.
    fn create_widget(&self, _: u32, _: ComRef<'_, IWidgetHostAbi>, _: NcCopyFromBufferOffset<'_, u8>, _: neocogi_com::NcValueRef<'_, neocogi_ui_document::contracts::WidgetEventBinding>) -> Result<neocogi_com::ComBox<IWidgetAbi>, NcStatus> {
        panic!("catalog inspection must not construct widgets")
    }
}

/// Owns the catalog object for the registry's ordinary COM discovery call.
fn register(registry: &mut CustomWidgetRegistry, manifest: &DesignWidgetManifest) -> Result<(), String> {
    let catalog = Catalog::from_value(Catalog {
        bytes: serde_json::to_vec(manifest).unwrap(),
    });
    registry.register_catalog(catalog.query::<IDesignWidgetCatalogAbi>().unwrap())
}

/// Publishes two valid spellings and verifies malformed external paths fail before export.
fn main() {
    let destination = std::path::PathBuf::from(std::env::args_os().nth(1).unwrap());
    let mut manifest = DesignWidgetManifest::from_idl("example.counter", vec![], &CounterAbi::IDL_INTERFACE).unwrap();
    assert_eq!(manifest.interface.rust_abi, "path_contracts::r#type::CounterAbi");
    assert_eq!(manifest.interface.event_value().unwrap().rust_type, "path_contracts::r#type::CounterEvent");
    for file in ["plain.rs", "absolute.rs"] {
        if file == "absolute.rs" {
            manifest.interface.rust_abi.insert_str(0, "::");
            for value in &mut manifest.interface.values {
                value.rust_type.insert_str(0, "::");
            }
        }
        let mut registry = CustomWidgetRegistry::new();
        register(&mut registry, &manifest).unwrap();
        let mut document = Document::new("paths");
        document.outputs.clear();
        document.root = Node::new("counter", registry.default_spec("example.counter").unwrap());
        std::fs::write(destination.join(file), document.rust_bindings_with(&registry).unwrap()).unwrap();
    }
    // COM transport accepts arbitrary producer bytes. Both admission paths must
    // reject invalid Rust syntax, and a failed discovery must publish no kinds.
    for interface in [true, false] {
        let mut invalid = manifest.clone();
        let field = match interface {
            true => {
                invalid.interface.rust_abi = "path_contracts::type::CounterAbi".into();
                "interface.rust_abi"
            }
            false => {
                invalid.interface.values[0].rust_type = "path_contracts::type::CounterEvent".into();
                "interface.values[0].rust_type"
            }
        };
        let mut registry = CustomWidgetRegistry::new();
        let error = register(&mut registry, &invalid).unwrap_err();
        assert!(error.contains("example.counter") && error.contains(field), "{error}");
        assert_eq!(registry.kinds().count(), 0);
    }
}
