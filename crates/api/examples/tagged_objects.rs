use neocogi_com::{ComPtr, IUnknown, NcStatus, com_object, com_enum, com_int_enum, com_interface};

com_int_enum! {
    pub enum ElementKind: u16 {
        ROOT = 1,
        BUTTON = 2,
        LABEL = 3,
    }
}

#[com_interface(auto_guid = "nc.examples.tagged_objects.Element:v1", assigned_signature = b"00000000000000000000000000000032")]
pub trait IElement {
    fn id(&self) -> Result<u64, NcStatus>;
    fn kind(&self) -> Result<ElementKind, NcStatus>;
}

#[com_interface(auto_guid = "nc.examples.tagged_objects.Document:v1", assigned_signature = b"00000000000000000000000000000033")]
pub trait IDocument {
    fn root_id(&self) -> Result<u64, NcStatus>;
}

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

#[com_enum(tag = u32, assigned_signature = b"00000000000000000000000000000034")]
#[derive(Debug)]
pub enum Selection {
    Empty,
    Element { id: u64, kind: ElementKind },
    Document { root_id: u64 },
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
    let root = Element::create_element(1, ElementKind::ROOT);
    let button = Element::create_element(2, ElementKind::BUTTON);
    let document = Document::create_document(root.clone());

    // Generated COM APIs now accept these native values directly. Their
    // checked wire codec is internal to the call boundary.
    describe(Selection::Element { id: button.id()?, kind: button.kind()? })?;
    describe(Selection::Document { root_id: document.root_id()? })?;
    describe(Selection::Empty)?;

    Ok(())
}
