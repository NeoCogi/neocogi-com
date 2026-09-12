use neocogi_com::{ComPtr, IUnknown, NcStatus, com_object, com_enum, com_int_enum, com_interface};

com_int_enum! {
    /// Stable node kinds used by this example's COM ABI.
    pub enum NodeKind: u16 {
        DOCUMENT = 1,
        BODY = 2,
        BUTTON = 3,
        SPAN = 4,
    }
}

#[com_enum(tag = u32, assigned_signature = b"00000000000000000000000000000029")]
#[derive(Debug, PartialEq)]
pub enum TreeMutation {
    None,
    ChildAdded(u64, NodeKind),
    AttributeChanged { element: u64, name_id: u32 },
}

#[com_interface(auto_guid = "nc.examples.tree.Element:v1", assigned_signature = b"00000000000000000000000000000030")]
pub trait IElement {
    fn kind(&self) -> Result<NodeKind, NcStatus>;
    fn child_count(&self) -> Result<i32, NcStatus>;
    fn first_child_kind(&self) -> Result<NodeKind, NcStatus>;
}

#[com_interface(auto_guid = "nc.examples.tree.Document:v1", assigned_signature = b"00000000000000000000000000000031")]
pub trait IDocument {
    fn root_kind(&self) -> Result<NodeKind, NcStatus>;
    fn root_child_count(&self) -> Result<i32, NcStatus>;
}

#[com_object(interfaces(IElement))]
pub struct Element {
    kind: NodeKind,
    children: Vec<ComPtr<IElementAbi>>,
}

impl Element {
    fn create_element(kind: NodeKind, children: Vec<ComPtr<IElementAbi>>) -> ComPtr<IElementAbi> {
        let unknown: ComPtr<IUnknown> = Element::from_value(Self { kind, children });
        unknown.query::<IElementAbi>().expect("Element exposes IElement")
    }
}

impl IElement for Element {
    fn kind(&self) -> Result<NodeKind, NcStatus> {
        Ok(self.kind)
    }

    fn child_count(&self) -> Result<i32, NcStatus> {
        Ok(self.children.len() as i32)
    }

    fn first_child_kind(&self) -> Result<NodeKind, NcStatus> {
        match self.children.first() {
            Some(child) => child.kind(),
            None => Ok(NodeKind::from_raw(0)),
        }
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
    fn root_kind(&self) -> Result<NodeKind, NcStatus> {
        self.root.kind()
    }

    fn root_child_count(&self) -> Result<i32, NcStatus> {
        self.root.child_count()
    }
}

fn main() -> Result<(), NcStatus> {
    let button = Element::create_element(NodeKind::BUTTON, Vec::new());
    let span = Element::create_element(NodeKind::SPAN, Vec::new());
    let body = Element::create_element(NodeKind::BODY, vec![button.clone(), span.clone()]);
    let document = Document::create_document(body.clone());

    println!("document root kind = {:?}", document.root_kind()?);
    println!("document root child count = {}", document.root_child_count()?);
    println!("body first child kind = {:?}", body.first_child_kind()?);
    println!("button child count = {}", button.child_count()?);

    let body_unknown = body.query::<IUnknown>()?;
    println!("body as IUnknown: {body_unknown:?}");

    let body_again = body_unknown.query::<IElementAbi>()?;
    println!("IUnknown -> IElementAbi kind = {:?}", body_again.kind()?);

    let future_kind = NodeKind::from_raw(99);
    println!("future node kind {} is known? {}", future_kind.raw(), future_kind.is_known());

    let mutation = TreeMutation::AttributeChanged { element: 42, name_id: 7 };
    let mutation_abi = TreeMutationAbi::from(mutation);
    println!("mutation ABI tag = {}", mutation_abi.tag().raw());
    println!("mutation round trip = {:?}", TreeMutation::try_from(mutation_abi));

    let _ = NodeKind::DOCUMENT;
    Ok(())
}
