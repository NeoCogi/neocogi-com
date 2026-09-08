use nc_com::{ComPtr, IUnknown, NcStatus, com_object, com_enum, com_int_enum, com_interface};

com_int_enum! {
    pub enum ElementKind: u16 {
        ROOT = 1,
        BUTTON = 2,
        LABEL = 3,
    }
}

#[com_interface(auto_guid = "nc.examples.tagged_objects.Element:v1")]
pub trait IElement {
    fn id(&self) -> Result<u64, NcStatus>;
    fn kind(&self) -> Result<ElementKind, NcStatus>;
}

#[com_interface(auto_guid = "nc.examples.tagged_objects.Document:v1")]
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
        unknown
            .query::<IElementAbi>()
            .expect("Element exposes IElement")
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
        unknown
            .query::<IDocumentAbi>()
            .expect("Document exposes IDocument")
    }
}

impl IDocument for Document {
    fn root_id(&self) -> Result<u64, NcStatus> {
        self.root.id()
    }
}

#[com_enum(tag = u32)]
#[derive(Debug)]
pub enum Selection {
    #[com_tag(1)]
    Empty,
    #[com_tag(2)]
    Element { id: u64, kind: ElementKind },
    #[com_tag(3)]
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

    let element_selection = Selection::Element {
        id: button.id()?,
        kind: button.kind()?,
    };
    let element_abi = SelectionAbi::from(element_selection);
    println!("element selection ABI tag = {}", element_abi.tag().raw());
    describe(Selection::try_from(element_abi)?)?;

    let document_selection = Selection::Document {
        root_id: document.root_id()?,
    };
    let document_abi = SelectionAbi::from(document_selection);
    println!("document selection ABI tag = {}", document_abi.tag().raw());
    describe(Selection::try_from(document_abi)?)?;

    let empty_abi = SelectionAbi::from(Selection::Empty);
    println!("empty selection ABI tag = {}", empty_abi.tag().raw());
    describe(Selection::try_from(empty_abi)?)?;

    Ok(())
}
