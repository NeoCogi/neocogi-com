use neocogi_com::{ComPtr, IUnknown, NcStatus, com_object};

// Contract identity and wire codecs are compiled from examples/schemas/nc/examples/tagged_objects.ncidl.
#[allow(dead_code, unused_imports)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/example_tagged_objects.rs"));
}
use bindings::*;

#[com_object(interfaces(IElement))]
pub struct Element {
    id: u64,
    kind: ElementKind,
}

impl Element {
    fn create_element(id: u64, kind: ElementKind) -> ComPtr<IElementAbi> {
        let unknown: ComPtr<IUnknown> = Element::from_value(Self { id, kind });
        unknown.query::<IElementAbi>().expect("Element exposes IElement")
    }
}

impl IElement for Element {
    fn id(&self) -> Result<u64, NcStatus> {
        Ok(self.id)
    }

    fn kind(&self) -> Result<ElementKind, NcStatus> {
        Ok(self.kind)
    }
}

#[com_object(interfaces(IDocument))]
pub struct Document {
    root: ComPtr<IElementAbi>,
}

impl Document {
    fn create_document(root: ComPtr<IElementAbi>) -> ComPtr<IDocumentAbi> {
        let unknown: ComPtr<IUnknown> = Document::from_value(Self { root });
        unknown.query::<IDocumentAbi>().expect("Document exposes IDocument")
    }
}

impl IDocument for Document {
    fn root_id(&self) -> Result<u64, NcStatus> {
        self.root.id()
    }
}

fn describe(selection: Selection) -> Result<(), NcStatus> {
    match selection {
        Selection::Empty => println!("selection is empty"),
        Selection::Element { id, kind } => {
            println!("selection is element id={id} kind={kind:?}");
        }
        Selection::Document { root_id } => {
            println!("selection is document root={root_id}");
        }
    }
    Ok(())
}

fn main() -> Result<(), NcStatus> {
    let root = Element::create_element(1, ElementKind::Root);
    let button = Element::create_element(2, ElementKind::Button);
    let document = Document::create_document(root.clone());

    // Generated COM APIs now accept these native values directly. Their
    // checked wire codec is internal to the call boundary.
    describe(Selection::Element { id: button.id()?, kind: button.kind()? })?;
    describe(Selection::Document { root_id: document.root_id()? })?;
    describe(Selection::Empty)?;

    Ok(())
}
