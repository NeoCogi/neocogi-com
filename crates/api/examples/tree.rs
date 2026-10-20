use neocogi_com::{ComValue, ComPtr, IUnknown, NcStatus, com_object};

// Contract identity and wire codecs are compiled from examples/idl/nc/examples/tree.ncidl.
#[allow(dead_code, unused_imports)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/example_tree.rs"));
}
use bindings::*;

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
    let button = Element::create_element(NodeKind::Button, Vec::new());
    let span = Element::create_element(NodeKind::Span, Vec::new());
    let body = Element::create_element(NodeKind::Body, vec![button.clone(), span.clone()]);
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
    let (wire, _storage) = mutation.encode_value(&mut Default::default())?;
    println!("mutation wire tag = {}", wire.tag);
    // SAFETY: the checked encoder supplied this self-contained raw value.
    let decoded = unsafe { TreeMutation::decode_value(wire, &mut Default::default()) }?;
    println!("mutation round trip = {decoded:?}");

    let _ = NodeKind::Document;
    Ok(())
}
