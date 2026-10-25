// Real generated-code fixture, included by the active compiler's downstream test.
#[cfg(test)]
mod unit_tests {
    use super::*;
    use neocogi_com::*;
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

    /// Concrete state counts dispatch after validation and returned owner cleanup.
    #[com_object(IUnits)]
    struct Units { accepted: usize, drops: Arc<AtomicUsize> }
    /// A real returned owner must be released if another output is malformed.
    #[com_object(neocogi_com::persistence::IObjectChildren)]
    struct Item { drops: Arc<AtomicUsize> }
    impl neocogi_com::persistence::IObjectChildren for Item {
        fn count(&self)->Result<u32,NcStatus> { Ok(0) }
        fn get(&self,_:u32)->Result<ComPtr<IUnknown>,NcStatus> { Err(NC_E_OUT_OF_RANGE) }
    }
    impl Drop for Item { fn drop(&mut self) { self.drops.fetch_add(1,Ordering::SeqCst); } }
    impl IUnits for Units {
        fn status(&self)->Result<(),NcStatus> { Ok(()) }
        fn completion(&mut self,value:Completion)->Result<Completion,NcStatus> { self.accepted+=1; Ok(value) }
        fn optional(&self,value:Maybe)->Result<Maybe,NcStatus> { Ok(value) }
        fn mixed(&mut self,value:Mixed)->Result<Mixed,NcStatus> { self.accepted+=1; Ok(value) }
        fn arrays(&self,value:Arrays)->Result<Arrays,NcStatus> { Ok(value) }
        fn direct(&self,value:[();2])->Result<[();2],NcStatus> { Ok(value) }
        fn buffers(&mut self,input:NcCopyFromBufferOffset<'_,()>,mut output:NcOutputWriter<'_,()>)->Result<(),NcStatus> {
            self.accepted+=1; output.extend_from_slice(input.as_slice())
        }
        fn finish(&self,mut output:NcOutputWriter<'_,Completion>)->Result<ComPtr<IUnknown>,NcStatus> {
            output.push(Completion::Done(()))?;
            Ok(Item::from_value(Item { drops:self.drops.clone() }))
        }
    }

    /// Forwards a successful real provider call, then corrupts an initialized
    /// active wire byte. It leaves the successful owned result intact.
    unsafe extern "C" fn malformed_finish(this:*mut core::ffi::c_void,offset:u64,count:u64,
        output:*mut <Completion as ComValue>::Wire,written:*mut u64,out:*mut ComRaw<IUnknown>)->NcStatus {
        // SAFETY: this table is paired only with Units; all storage is the
        // generated client's real prepared output and return slots.
        let status=unsafe { (IUnitsVtblFor::<Units>::VTABLE.finish)(this,offset,count,output,written,out) };
        if status==NC_OK {
            // SAFETY: successful finish wrote one Done wire in nonempty storage.
            unsafe { (*output).payload.case_0.field_0=7; }
        }
        status
    }
    /// Only the output corruption differs from the exact generated contract.
    static MALFORMED:IUnitsVTable=IUnitsVTable { finish:malformed_finish,..IUnitsVtblFor::<Units>::VTABLE };

    /// Both the generated codec and the actual provider gate participate.
    #[test]
    fn unit_calls_and_malformed_storage() {
        const { assert!(!<()>::BORROW_WIRE); assert!(!<[();2]>::BORROW_WIRE); assert!(!Mixed::BORROW_WIRE); }
        let drops=Arc::new(AtomicUsize::new(0));
        let local=Units::into_local(Units { accepted:0,drops:drops.clone() });
        let mut owner:ComPtr<IUnitsAbi>=local.query().unwrap();
        owner.status().unwrap();
        assert_eq!(owner.completion(Completion::Done(())).unwrap(),Completion::Done(()));
        assert_eq!(owner.optional(Maybe::Some(())).unwrap(),Maybe::Some(()));
        assert_eq!(owner.optional(Maybe::None).unwrap(),Maybe::None);
        let mixed=Mixed { before:3,unit:(),enabled:true,after:7 };
        assert_eq!(owner.mixed(mixed).unwrap(),mixed);
        let arrays=Arrays { nested:[[();2];2],empty:[] };
        assert_eq!(owner.arrays(arrays).unwrap(),arrays);
        assert_eq!(owner.direct([();2]).unwrap(),[();2]);
        let input=[();3];
        let encoded=EncodedInput::new(NcCopyFromBufferOffset::new(0,&input)).unwrap();
        assert_ne!(encoded.as_ptr().cast::<()>(), input.as_ptr());
        let mut output=NcOutputBuffer::new();
        owner.buffers(NcCopyFromBufferOffset::new(0,&input),output.output(0,3)).unwrap();
        assert_eq!(output.as_slice(),input);
        let accepted=local.try_borrow().unwrap().accepted;
        let raw=owner.as_raw();
        let mut result=core::mem::MaybeUninit::uninit();
        let mut bad=encode_inline(&Completion::Done(())).unwrap();
        bad.payload.case_0.field_0=1;
        // SAFETY: initialized, live raw storage; no inactive member is read.
        unsafe {
            assert_eq!(((*raw.vtable).completion)(raw.object,bad,result.as_mut_ptr()),NC_E_INVALID_ARG);
            bad.tag=99;
            assert_eq!(((*raw.vtable).completion)(raw.object,bad,result.as_mut_ptr()),NC_E_INVALID_TAG);
        }
        for invalid_unit in [false,true] {
            let mut bad=encode_inline(&mixed).unwrap();
            if invalid_unit { bad.unit=2; } else { bad.enabled=2; }
            let mut result=core::mem::MaybeUninit::uninit();
            // SAFETY: only legal u8 storage contains a semantically bad leaf.
            assert_eq!(unsafe { ((*raw.vtable).mixed)(raw.object,bad,result.as_mut_ptr()) },NC_E_INVALID_ARG);
        }
        let invalid=[0u8,4,0];
        let mut wire_output=[99u8;3];
        let mut written=99;
        assert_eq!(unsafe { ((*raw.vtable).buffers)(raw.object,0,3,invalid.as_ptr(),0,3,wire_output.as_mut_ptr(),&mut written) },NC_E_INVALID_ARG);
        assert_eq!(wire_output,[99;3]);
        assert_eq!(local.try_borrow().unwrap().accepted,accepted);
        let (_,guard)=mixed.encode_value(&mut ValueBudget::new(0,5,2)).unwrap();
        let _=guard;
        assert!(matches!(mixed.encode_value(&mut ValueBudget::new(0,4,2)),Err(NC_E_OUT_OF_RANGE)));
        assert!(matches!([();2].encode_value(&mut ValueBudget::new(0,2,2)),Err(NC_E_OUT_OF_RANGE)));
        assert_eq!(core::mem::size_of::<<() as ComValue>::Wire>(),1);
        assert_eq!(core::mem::size_of::<()>(),0);
        assert_ne!(<()>::ABI_TYPE,NcAbiType::Void);
        let table=&IUnitsVtblFor::<Units>::VTABLE;
        let _:unsafe extern "C" fn(*mut core::ffi::c_void)->NcStatus=table.status;
        let _:unsafe extern "C" fn(*mut core::ffi::c_void,<Completion as ComValue>::Wire,*mut <Completion as ComValue>::Wire)->NcStatus=table.completion;
        assert!(matches!(Completion::ABI_TYPE,NcAbiType::Aggregate(_)));
        assert_eq!(Mixed::IDL_TYPE.shape, neocogi_com::idl_metadata::TypeShape::Struct(&[
            neocogi_com::idl_metadata::FieldMetadata { name:"before",ty:neocogi_com::idl_metadata::ValueType::Scalar("u32") },
            neocogi_com::idl_metadata::FieldMetadata { name:"unit",ty:neocogi_com::idl_metadata::ValueType::Unit },
            neocogi_com::idl_metadata::FieldMetadata { name:"enabled",ty:neocogi_com::idl_metadata::ValueType::Scalar("bool") },
            neocogi_com::idl_metadata::FieldMetadata { name:"after",ty:neocogi_com::idl_metadata::ValueType::Scalar("u32") },
        ]));
        let pair=owner.into_raw();
        // SAFETY: exact same object/identity/thunks, with one checked output
        // corrupted after successful provider dispatch; consumes the owner token.
        let malformed=unsafe { ComPtr::<IUnitsAbi>::from_raw_owned(ComRaw { object:pair.object,vtable:&MALFORMED }) }.unwrap();
        let mut destination=NcOutputBuffer::new();
        let returned=malformed.finish(destination.output(0,1));
        assert_eq!(returned.err(),Some(NC_E_INVALID_ARG));
        assert!(destination.is_empty());
        assert_eq!(drops.load(Ordering::SeqCst),1);
    }
}
