//! Identical providers use freshly generated or checked-in contracts without sharing declarations.
#[allow(dead_code)]
mod peer;
use neocogi_com::{com_object, ComPtr, ComRef, IUnknown, INcBytesAbi, NcBytes, NcCopyFromBufferOffset, NcGuid, NcStatus, NC_E_INVALID_ARG, NC_E_OUT_OF_RANGE};
use neocogi_com::persistence as builtin;

/// One exact initialization value passed through both interface representations.
const STATE: &[u8] = b"current persistence peer";
/// Persistent recipe identity, independent of the COM contract identities.
const KIND: NcGuid = neocogi_com::guid_from_qualified_name("test.persistence.peers");

/// Supplies the same behavior through each module's independently defined vtables.
macro_rules! provider {
    ($name:ident, $api:ident) => {
        #[com_object(interfaces($api::IObjectChildren, $api::IPersistable, $api::IPersistenceFactory))]
        /// A shared leaf, empty construction list and stateless recipe provider.
        struct $name;
        impl $api::IObjectChildren for $name {
            /// Empty lists expose no child owner to either caller.
            fn count(&self) -> Result<u32, NcStatus> {
                Ok(0)
            }
            /// Out-of-range errors remain ordinary errors after exact negotiation.
            fn get(&self, _: u32) -> Result<ComPtr<IUnknown>, NcStatus> {
                Err(NC_E_OUT_OF_RANGE)
            }
        }
        impl $api::IPersistable for $name {
            /// Recipe identity is application policy rather than interface identity.
            fn type_id(&self) -> Result<NcGuid, NcStatus> {
                Ok(KIND)
            }
            /// Both peers observe the same explicit initialization revision.
            fn schema_version(&self) -> Result<u32, NcStatus> {
                Ok(7)
            }
            /// Returned state owns bytes through the ordinary core buffer capability.
            fn save_state(&self) -> Result<ComPtr<INcBytesAbi>, NcStatus> {
                Ok(NcBytes::from_slice(STATE))
            }
            /// This fixture has no owned descendants.
            fn child_count(&self) -> Result<u32, NcStatus> {
                Ok(0)
            }
            /// No index can select a descendant in an empty tree.
            fn child(&self, _: u32) -> Result<ComPtr<IUnknown>, NcStatus> {
                Err(NC_E_OUT_OF_RANGE)
            }
        }
        impl $api::IPersistenceFactory for $name {
            /// Validate actual transported byte contents and copied arguments.
            fn validate(&self, version: u32, state: NcCopyFromBufferOffset<'_, u8>, children: u32) -> Result<(), NcStatus> {
                if version != 7 || state.offset() != 0 || state.as_slice() != STATE || children != 0 {
                    return Err(NC_E_INVALID_ARG);
                }
                Ok(())
            }
            /// Calls through the borrowed peer children contract before returning a shared root.
            fn restore(
                &self,
                version: u32,
                state: NcCopyFromBufferOffset<'_, u8>,
                children: ComRef<'_, $api::IObjectChildrenAbi>,
            ) -> Result<ComPtr<IUnknown>, NcStatus> {
                self.validate(version, state, children.count()?)?;
                Ok(Self::from_value(Self))
            }
        }
    };
}
provider!(Builtin, builtin);
provider!(Fresh, peer);

/// Tests both identity negotiation and real getter/buffer/borrowed-result ABI operations.
macro_rules! exercise {
    ($root:expr, $api:ident) => {{
        let root = $root;
        let children = root.query::<$api::IObjectChildrenAbi>().unwrap();
        assert_eq!(children.count().unwrap(), 0);
        assert_eq!(children.get(0).err(), Some(NC_E_OUT_OF_RANGE));
        let persistent = root.query::<$api::IPersistableAbi>().unwrap();
        assert_eq!(persistent.type_id().unwrap(), KIND);
        assert_eq!(persistent.schema_version().unwrap(), 7);
        assert_eq!(persistent.child_count().unwrap(), 0);
        assert_eq!(persistent.child(0).err(), Some(NC_E_OUT_OF_RANGE));
        use neocogi_com::INcBytesComPtrExt;
        assert_eq!(persistent.save_state().unwrap().to_vec().unwrap(), STATE);
        let factory = root.query::<$api::IPersistenceFactoryAbi>().unwrap();
        assert_eq!(factory.validate(7, NcCopyFromBufferOffset::new(0, STATE), 0), Ok(()));
        assert_eq!(factory.validate(8, NcCopyFromBufferOffset::new(0, STATE), 0), Err(NC_E_INVALID_ARG));
        let restored = factory.restore(7, NcCopyFromBufferOffset::new(0, STATE), children.as_com_ref()).unwrap();
        assert_eq!(restored.query::<$api::IPersistableAbi>().unwrap().schema_version().unwrap(), 7);
    }};
}

/// Freshly generated clients must query every checked-in provider contract successfully.
#[test]
fn fresh_clients_call_builtin_providers() {
    exercise!(Builtin::from_value(Builtin), peer);
}
/// Checked-in clients must likewise query and call providers implementing fresh declarations.
#[test]
fn builtin_clients_call_fresh_providers() {
    exercise!(Fresh::from_value(Fresh), builtin);
}
