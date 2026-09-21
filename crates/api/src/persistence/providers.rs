// Generated from core/idl/neocogi/com/persistence/objects.ncidl.
// Regenerate: NC_UPDATE_PERSISTENCE_BINDINGS=1 cargo test -p neocogi-com-macros persistence_bindings_match_idl

use super::*;
#[doc(hidden)]
/// Interface projections embedded in one generated provider allocation.
pub struct IObjectChildrenInterfacesFor<T> {
    /// Projection whose header advertises this exact interface contract.
    own: IObjectChildrenAbi,
    /// Associates the thunk implementor without owning a second value.
    implementor: core::marker::PhantomData<fn() -> T>,
}
impl<T: IObjectChildren + ::neocogi_com::ComAbiObject> IObjectChildrenInterfacesFor<T> {
    /// Creates unpublished headers; the object installs its back-pointer.
    pub const fn __nc_new() -> Self {
        Self {
            own: IObjectChildrenAbi {
                vtable: &IObjectChildrenVtblFor::<T>::VTABLE,
                object: core::ptr::null_mut(),
            },
            implementor: core::marker::PhantomData,
        }
    }
}
impl<T> IObjectChildrenInterfacesFor<T> {
    /// Patches every projection before the object is published.
    pub fn set_object(&mut self, object: *mut core::ffi::c_void) {
        self.own.object = object;
    }
    /// Finds a checked projection without changing the reference count.
    pub fn query(
        &self,
        interface: &::neocogi_com::NcInterfaceId,
    ) -> Result<*mut core::ffi::c_void, ::neocogi_com::NcStatus> {
        if interface.iid == <IObjectChildrenAbi as ::neocogi_com::ComInterface>::IID {
            if *interface
                != <IObjectChildrenAbi as ::neocogi_com::ComInterface>::INTERFACE_ID
            {
                return Err(::neocogi_com::NC_E_ABI_VERSION);
            }
            return Ok(core::ptr::from_ref(&self.own).cast_mut().cast());
        }
        Ok(core::ptr::null_mut())
    }
}
#[doc(hidden)]
/// Statically typed vtable and lifecycle adapters for one provider type.
pub struct IObjectChildrenVtblFor<T>(
    /// Associates dispatch with T without allocating or owning a value.
    core::marker::PhantomData<T>,
);
impl<T> IObjectChildrenVtblFor<T>
where
    T: IObjectChildren + ::neocogi_com::ComAbiObject,
{
    /// Table for a standalone projection of this interface.
    pub const VTABLE: IObjectChildrenVTable = Self::with_identity(
        &<IObjectChildrenAbi as ::neocogi_com::ComInterface>::INTERFACE_ID,
    );
    /// Builds an inherited prefix advertising the actual outer projection.
    pub const fn with_identity(
        identity: &'static ::neocogi_com::NcInterfaceId,
    ) -> IObjectChildrenVTable {
        IObjectChildrenVTable {
            unknown: ::neocogi_com::IUnknownVTable {
                interface_id: identity,
                query_interface: Self::query_interface_thunk,
                add_ref: Self::add_ref_thunk,
                release: Self::release_thunk,
            },
            count: Self::count_thunk,
            get: Self::get_thunk,
        }
    }
    /// Routes this projection to the allocation-wide interface lookup.
    ///
    /// # Safety
    /// The live header belongs to T; identity and output storage satisfy
    /// the QueryInterface contract. The owning allocation validates them.
    unsafe extern "C" fn query_interface_thunk(
        this: *mut ::neocogi_com::IUnknown,
        interface_id: *const ::neocogi_com::NcInterfaceId,
        out: *mut *mut core::ffi::c_void,
    ) -> ::neocogi_com::NcStatus {
        if this.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let object = unsafe { (*this).object };
        unsafe {
            <T as ::neocogi_com::ComAbiObject>::query_interface_from_abi_object(
                object,
                interface_id,
                out,
            )
        }
    }
    /// Retains the common allocation through this interface projection.
    ///
    /// # Safety
    /// Non-null this belongs to T's live generated object storage.
    unsafe extern "C" fn add_ref_thunk(this: *mut ::neocogi_com::IUnknown) -> u32 {
        if this.is_null() {
            return 0;
        }
        let object = unsafe { (*this).object };
        unsafe { <T as ::neocogi_com::ComAbiObject>::add_ref_from_abi_object(object) }
    }
    /// Releases the common allocation through this interface projection.
    ///
    /// # Safety
    /// Non-null this transfers one live reference from T's object storage.
    unsafe extern "C" fn release_thunk(this: *mut ::neocogi_com::IUnknown) -> u32 {
        if this.is_null() {
            return 0;
        }
        let object = unsafe { (*this).object };
        unsafe { <T as ::neocogi_com::ComAbiObject>::release_from_abi_object(object) }
    }
    /// Validates raw pointers, borrows the concrete provider through its
    /// object-wide gate, invokes its typed method, and lowers the result.
    /// The gate is released before transferring an owned result; errors
    /// are normalized so an unwritten output can never look successful.
    ///
    /// # Safety
    /// The interface belongs to T's live generated storage. Non-null ABI
    /// arguments must satisfy their method's readable/writable contracts.
    unsafe extern "C" fn count_thunk(
        this: *mut IObjectChildrenAbi,
        __nc_out: *mut u32,
    ) -> ::neocogi_com::NcStatus {
        if this.is_null() || __nc_out.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let __nc_dispatch_object = unsafe { (*this).object };
        if __nc_dispatch_object.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let mut __nc_dispatch_value = match unsafe {
            <T as ::neocogi_com::ComAbiObject>::borrow_abi_ref(__nc_dispatch_object)
        } {
            Ok(value) => value,
            Err(status) => return status,
        };
        let result = <T as IObjectChildren>::count(&*__nc_dispatch_value);
        drop(__nc_dispatch_value);
        match result {
            Ok(value) => {
                unsafe {
                    *__nc_out = value;
                }
                ::neocogi_com::NC_OK
            }
            Err(status) => status.normalize_error(),
        }
    }
    /// Validates raw pointers, borrows the concrete provider through its
    /// object-wide gate, invokes its typed method, and lowers the result.
    /// The gate is released before transferring an owned result; errors
    /// are normalized so an unwritten output can never look successful.
    ///
    /// # Safety
    /// The interface belongs to T's live generated storage. Non-null ABI
    /// arguments must satisfy their method's readable/writable contracts.
    unsafe extern "C" fn get_thunk(
        this: *mut IObjectChildrenAbi,
        index: u32,
        __nc_out: *mut *mut ::neocogi_com::IUnknown,
    ) -> ::neocogi_com::NcStatus {
        if this.is_null() || __nc_out.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let __nc_dispatch_object = unsafe { (*this).object };
        if __nc_dispatch_object.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let mut __nc_dispatch_value = match unsafe {
            <T as ::neocogi_com::ComAbiObject>::borrow_abi_ref(__nc_dispatch_object)
        } {
            Ok(value) => value,
            Err(status) => return status,
        };
        let result = <T as IObjectChildren>::get(&*__nc_dispatch_value, index);
        drop(__nc_dispatch_value);
        match result {
            Ok(value) => {
                unsafe {
                    *__nc_out = value.into_raw();
                }
                ::neocogi_com::NC_OK
            }
            Err(status) => status.normalize_error(),
        }
    }
}
#[doc(hidden)]
/// Interface projections embedded in one generated provider allocation.
pub struct IPersistableInterfacesFor<T> {
    /// Projection whose header advertises this exact interface contract.
    own: IPersistableAbi,
    /// Associates the thunk implementor without owning a second value.
    implementor: core::marker::PhantomData<fn() -> T>,
}
impl<T: IPersistable + ::neocogi_com::ComAbiObject> IPersistableInterfacesFor<T> {
    /// Creates unpublished headers; the object installs its back-pointer.
    pub const fn __nc_new() -> Self {
        Self {
            own: IPersistableAbi {
                vtable: &IPersistableVtblFor::<T>::VTABLE,
                object: core::ptr::null_mut(),
            },
            implementor: core::marker::PhantomData,
        }
    }
}
impl<T> IPersistableInterfacesFor<T> {
    /// Patches every projection before the object is published.
    pub fn set_object(&mut self, object: *mut core::ffi::c_void) {
        self.own.object = object;
    }
    /// Finds a checked projection without changing the reference count.
    pub fn query(
        &self,
        interface: &::neocogi_com::NcInterfaceId,
    ) -> Result<*mut core::ffi::c_void, ::neocogi_com::NcStatus> {
        if interface.iid == <IPersistableAbi as ::neocogi_com::ComInterface>::IID {
            if *interface
                != <IPersistableAbi as ::neocogi_com::ComInterface>::INTERFACE_ID
            {
                return Err(::neocogi_com::NC_E_ABI_VERSION);
            }
            return Ok(core::ptr::from_ref(&self.own).cast_mut().cast());
        }
        Ok(core::ptr::null_mut())
    }
}
#[doc(hidden)]
/// Statically typed vtable and lifecycle adapters for one provider type.
pub struct IPersistableVtblFor<T>(
    /// Associates dispatch with T without allocating or owning a value.
    core::marker::PhantomData<T>,
);
impl<T> IPersistableVtblFor<T>
where
    T: IPersistable + ::neocogi_com::ComAbiObject,
{
    /// Table for a standalone projection of this interface.
    pub const VTABLE: IPersistableVTable = Self::with_identity(
        &<IPersistableAbi as ::neocogi_com::ComInterface>::INTERFACE_ID,
    );
    /// Builds an inherited prefix advertising the actual outer projection.
    pub const fn with_identity(
        identity: &'static ::neocogi_com::NcInterfaceId,
    ) -> IPersistableVTable {
        IPersistableVTable {
            unknown: ::neocogi_com::IUnknownVTable {
                interface_id: identity,
                query_interface: Self::query_interface_thunk,
                add_ref: Self::add_ref_thunk,
                release: Self::release_thunk,
            },
            type_id: Self::type_id_thunk,
            schema_version: Self::schema_version_thunk,
            save_state: Self::save_state_thunk,
            child_count: Self::child_count_thunk,
            child: Self::child_thunk,
        }
    }
    /// Routes this projection to the allocation-wide interface lookup.
    ///
    /// # Safety
    /// The live header belongs to T; identity and output storage satisfy
    /// the QueryInterface contract. The owning allocation validates them.
    unsafe extern "C" fn query_interface_thunk(
        this: *mut ::neocogi_com::IUnknown,
        interface_id: *const ::neocogi_com::NcInterfaceId,
        out: *mut *mut core::ffi::c_void,
    ) -> ::neocogi_com::NcStatus {
        if this.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let object = unsafe { (*this).object };
        unsafe {
            <T as ::neocogi_com::ComAbiObject>::query_interface_from_abi_object(
                object,
                interface_id,
                out,
            )
        }
    }
    /// Retains the common allocation through this interface projection.
    ///
    /// # Safety
    /// Non-null this belongs to T's live generated object storage.
    unsafe extern "C" fn add_ref_thunk(this: *mut ::neocogi_com::IUnknown) -> u32 {
        if this.is_null() {
            return 0;
        }
        let object = unsafe { (*this).object };
        unsafe { <T as ::neocogi_com::ComAbiObject>::add_ref_from_abi_object(object) }
    }
    /// Releases the common allocation through this interface projection.
    ///
    /// # Safety
    /// Non-null this transfers one live reference from T's object storage.
    unsafe extern "C" fn release_thunk(this: *mut ::neocogi_com::IUnknown) -> u32 {
        if this.is_null() {
            return 0;
        }
        let object = unsafe { (*this).object };
        unsafe { <T as ::neocogi_com::ComAbiObject>::release_from_abi_object(object) }
    }
    /// Validates raw pointers, borrows the concrete provider through its
    /// object-wide gate, invokes its typed method, and lowers the result.
    /// The gate is released before transferring an owned result; errors
    /// are normalized so an unwritten output can never look successful.
    ///
    /// # Safety
    /// The interface belongs to T's live generated storage. Non-null ABI
    /// arguments must satisfy their method's readable/writable contracts.
    unsafe extern "C" fn type_id_thunk(
        this: *mut IPersistableAbi,
        __nc_out: *mut ::neocogi_com::NcGuid,
    ) -> ::neocogi_com::NcStatus {
        if this.is_null() || __nc_out.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let __nc_dispatch_object = unsafe { (*this).object };
        if __nc_dispatch_object.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let mut __nc_dispatch_value = match unsafe {
            <T as ::neocogi_com::ComAbiObject>::borrow_abi_ref(__nc_dispatch_object)
        } {
            Ok(value) => value,
            Err(status) => return status,
        };
        let result = <T as IPersistable>::type_id(&*__nc_dispatch_value);
        drop(__nc_dispatch_value);
        match result {
            Ok(value) => {
                unsafe {
                    *__nc_out = value;
                }
                ::neocogi_com::NC_OK
            }
            Err(status) => status.normalize_error(),
        }
    }
    /// Validates raw pointers, borrows the concrete provider through its
    /// object-wide gate, invokes its typed method, and lowers the result.
    /// The gate is released before transferring an owned result; errors
    /// are normalized so an unwritten output can never look successful.
    ///
    /// # Safety
    /// The interface belongs to T's live generated storage. Non-null ABI
    /// arguments must satisfy their method's readable/writable contracts.
    unsafe extern "C" fn schema_version_thunk(
        this: *mut IPersistableAbi,
        __nc_out: *mut u32,
    ) -> ::neocogi_com::NcStatus {
        if this.is_null() || __nc_out.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let __nc_dispatch_object = unsafe { (*this).object };
        if __nc_dispatch_object.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let mut __nc_dispatch_value = match unsafe {
            <T as ::neocogi_com::ComAbiObject>::borrow_abi_ref(__nc_dispatch_object)
        } {
            Ok(value) => value,
            Err(status) => return status,
        };
        let result = <T as IPersistable>::schema_version(&*__nc_dispatch_value);
        drop(__nc_dispatch_value);
        match result {
            Ok(value) => {
                unsafe {
                    *__nc_out = value;
                }
                ::neocogi_com::NC_OK
            }
            Err(status) => status.normalize_error(),
        }
    }
    /// Validates raw pointers, borrows the concrete provider through its
    /// object-wide gate, invokes its typed method, and lowers the result.
    /// The gate is released before transferring an owned result; errors
    /// are normalized so an unwritten output can never look successful.
    ///
    /// # Safety
    /// The interface belongs to T's live generated storage. Non-null ABI
    /// arguments must satisfy their method's readable/writable contracts.
    unsafe extern "C" fn save_state_thunk(
        this: *mut IPersistableAbi,
        __nc_out: *mut *mut ::neocogi_com::INcBytesAbi,
    ) -> ::neocogi_com::NcStatus {
        if this.is_null() || __nc_out.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let __nc_dispatch_object = unsafe { (*this).object };
        if __nc_dispatch_object.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let mut __nc_dispatch_value = match unsafe {
            <T as ::neocogi_com::ComAbiObject>::borrow_abi_ref(__nc_dispatch_object)
        } {
            Ok(value) => value,
            Err(status) => return status,
        };
        let result = <T as IPersistable>::save_state(&*__nc_dispatch_value);
        drop(__nc_dispatch_value);
        match result {
            Ok(value) => {
                unsafe {
                    *__nc_out = value.into_raw();
                }
                ::neocogi_com::NC_OK
            }
            Err(status) => status.normalize_error(),
        }
    }
    /// Validates raw pointers, borrows the concrete provider through its
    /// object-wide gate, invokes its typed method, and lowers the result.
    /// The gate is released before transferring an owned result; errors
    /// are normalized so an unwritten output can never look successful.
    ///
    /// # Safety
    /// The interface belongs to T's live generated storage. Non-null ABI
    /// arguments must satisfy their method's readable/writable contracts.
    unsafe extern "C" fn child_count_thunk(
        this: *mut IPersistableAbi,
        __nc_out: *mut u32,
    ) -> ::neocogi_com::NcStatus {
        if this.is_null() || __nc_out.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let __nc_dispatch_object = unsafe { (*this).object };
        if __nc_dispatch_object.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let mut __nc_dispatch_value = match unsafe {
            <T as ::neocogi_com::ComAbiObject>::borrow_abi_ref(__nc_dispatch_object)
        } {
            Ok(value) => value,
            Err(status) => return status,
        };
        let result = <T as IPersistable>::child_count(&*__nc_dispatch_value);
        drop(__nc_dispatch_value);
        match result {
            Ok(value) => {
                unsafe {
                    *__nc_out = value;
                }
                ::neocogi_com::NC_OK
            }
            Err(status) => status.normalize_error(),
        }
    }
    /// Validates raw pointers, borrows the concrete provider through its
    /// object-wide gate, invokes its typed method, and lowers the result.
    /// The gate is released before transferring an owned result; errors
    /// are normalized so an unwritten output can never look successful.
    ///
    /// # Safety
    /// The interface belongs to T's live generated storage. Non-null ABI
    /// arguments must satisfy their method's readable/writable contracts.
    unsafe extern "C" fn child_thunk(
        this: *mut IPersistableAbi,
        index: u32,
        __nc_out: *mut *mut ::neocogi_com::IUnknown,
    ) -> ::neocogi_com::NcStatus {
        if this.is_null() || __nc_out.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let __nc_dispatch_object = unsafe { (*this).object };
        if __nc_dispatch_object.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let mut __nc_dispatch_value = match unsafe {
            <T as ::neocogi_com::ComAbiObject>::borrow_abi_ref(__nc_dispatch_object)
        } {
            Ok(value) => value,
            Err(status) => return status,
        };
        let result = <T as IPersistable>::child(&*__nc_dispatch_value, index);
        drop(__nc_dispatch_value);
        match result {
            Ok(value) => {
                unsafe {
                    *__nc_out = value.into_raw();
                }
                ::neocogi_com::NC_OK
            }
            Err(status) => status.normalize_error(),
        }
    }
}
#[doc(hidden)]
/// Interface projections embedded in one generated provider allocation.
pub struct IPersistenceFactoryInterfacesFor<T> {
    /// Projection whose header advertises this exact interface contract.
    own: IPersistenceFactoryAbi,
    /// Associates the thunk implementor without owning a second value.
    implementor: core::marker::PhantomData<fn() -> T>,
}
impl<
    T: IPersistenceFactory + ::neocogi_com::ComAbiObject,
> IPersistenceFactoryInterfacesFor<T> {
    /// Creates unpublished headers; the object installs its back-pointer.
    pub const fn __nc_new() -> Self {
        Self {
            own: IPersistenceFactoryAbi {
                vtable: &IPersistenceFactoryVtblFor::<T>::VTABLE,
                object: core::ptr::null_mut(),
            },
            implementor: core::marker::PhantomData,
        }
    }
}
impl<T> IPersistenceFactoryInterfacesFor<T> {
    /// Patches every projection before the object is published.
    pub fn set_object(&mut self, object: *mut core::ffi::c_void) {
        self.own.object = object;
    }
    /// Finds a checked projection without changing the reference count.
    pub fn query(
        &self,
        interface: &::neocogi_com::NcInterfaceId,
    ) -> Result<*mut core::ffi::c_void, ::neocogi_com::NcStatus> {
        if interface.iid == <IPersistenceFactoryAbi as ::neocogi_com::ComInterface>::IID
        {
            if *interface
                != <IPersistenceFactoryAbi as ::neocogi_com::ComInterface>::INTERFACE_ID
            {
                return Err(::neocogi_com::NC_E_ABI_VERSION);
            }
            return Ok(core::ptr::from_ref(&self.own).cast_mut().cast());
        }
        Ok(core::ptr::null_mut())
    }
}
#[doc(hidden)]
/// Statically typed vtable and lifecycle adapters for one provider type.
pub struct IPersistenceFactoryVtblFor<T>(
    /// Associates dispatch with T without allocating or owning a value.
    core::marker::PhantomData<T>,
);
impl<T> IPersistenceFactoryVtblFor<T>
where
    T: IPersistenceFactory + ::neocogi_com::ComAbiObject,
{
    /// Table for a standalone projection of this interface.
    pub const VTABLE: IPersistenceFactoryVTable = Self::with_identity(
        &<IPersistenceFactoryAbi as ::neocogi_com::ComInterface>::INTERFACE_ID,
    );
    /// Builds an inherited prefix advertising the actual outer projection.
    pub const fn with_identity(
        identity: &'static ::neocogi_com::NcInterfaceId,
    ) -> IPersistenceFactoryVTable {
        IPersistenceFactoryVTable {
            unknown: ::neocogi_com::IUnknownVTable {
                interface_id: identity,
                query_interface: Self::query_interface_thunk,
                add_ref: Self::add_ref_thunk,
                release: Self::release_thunk,
            },
            validate: Self::validate_thunk,
            restore: Self::restore_thunk,
        }
    }
    /// Routes this projection to the allocation-wide interface lookup.
    ///
    /// # Safety
    /// The live header belongs to T; identity and output storage satisfy
    /// the QueryInterface contract. The owning allocation validates them.
    unsafe extern "C" fn query_interface_thunk(
        this: *mut ::neocogi_com::IUnknown,
        interface_id: *const ::neocogi_com::NcInterfaceId,
        out: *mut *mut core::ffi::c_void,
    ) -> ::neocogi_com::NcStatus {
        if this.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let object = unsafe { (*this).object };
        unsafe {
            <T as ::neocogi_com::ComAbiObject>::query_interface_from_abi_object(
                object,
                interface_id,
                out,
            )
        }
    }
    /// Retains the common allocation through this interface projection.
    ///
    /// # Safety
    /// Non-null this belongs to T's live generated object storage.
    unsafe extern "C" fn add_ref_thunk(this: *mut ::neocogi_com::IUnknown) -> u32 {
        if this.is_null() {
            return 0;
        }
        let object = unsafe { (*this).object };
        unsafe { <T as ::neocogi_com::ComAbiObject>::add_ref_from_abi_object(object) }
    }
    /// Releases the common allocation through this interface projection.
    ///
    /// # Safety
    /// Non-null this transfers one live reference from T's object storage.
    unsafe extern "C" fn release_thunk(this: *mut ::neocogi_com::IUnknown) -> u32 {
        if this.is_null() {
            return 0;
        }
        let object = unsafe { (*this).object };
        unsafe { <T as ::neocogi_com::ComAbiObject>::release_from_abi_object(object) }
    }
    /// Validates raw pointers, borrows the concrete provider through its
    /// object-wide gate, invokes its typed method, and lowers the result.
    /// The gate is released before transferring an owned result; errors
    /// are normalized so an unwritten output can never look successful.
    ///
    /// # Safety
    /// The interface belongs to T's live generated storage. Non-null ABI
    /// arguments must satisfy their method's readable/writable contracts.
    unsafe extern "C" fn validate_thunk(
        this: *mut IPersistenceFactoryAbi,
        version: u32,
        __nc_state_offset: u64,
        __nc_state_len: u64,
        __nc_state_ptr: *const u8,
        child_count: u32,
    ) -> ::neocogi_com::NcStatus {
        if this.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let __nc_dispatch_object = unsafe { (*this).object };
        if __nc_dispatch_object.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let state = match unsafe {
            ::neocogi_com::NcCopyFromBufferOffset::<
                u8,
            >::__nc_from_raw_parts(__nc_state_offset, __nc_state_ptr, __nc_state_len)
        } {
            Ok(value) => value,
            Err(status) => return status.normalize_error(),
        };
        let mut __nc_dispatch_value = match unsafe {
            <T as ::neocogi_com::ComAbiObject>::borrow_abi_ref(__nc_dispatch_object)
        } {
            Ok(value) => value,
            Err(status) => return status,
        };
        let result = <T as IPersistenceFactory>::validate(
            &*__nc_dispatch_value,
            version,
            state,
            child_count,
        );
        drop(__nc_dispatch_value);
        match result {
            Ok(()) => ::neocogi_com::NC_OK,
            Err(status) => status.normalize_error(),
        }
    }
    /// Validates raw pointers, borrows the concrete provider through its
    /// object-wide gate, invokes its typed method, and lowers the result.
    /// The gate is released before transferring an owned result; errors
    /// are normalized so an unwritten output can never look successful.
    ///
    /// # Safety
    /// The interface belongs to T's live generated storage. Non-null ABI
    /// arguments must satisfy their method's readable/writable contracts.
    unsafe extern "C" fn restore_thunk(
        this: *mut IPersistenceFactoryAbi,
        version: u32,
        __nc_state_offset: u64,
        __nc_state_len: u64,
        __nc_state_ptr: *const u8,
        children: *mut IObjectChildrenAbi,
        __nc_out: *mut *mut ::neocogi_com::IUnknown,
    ) -> ::neocogi_com::NcStatus {
        if this.is_null() || __nc_out.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let __nc_dispatch_object = unsafe { (*this).object };
        if __nc_dispatch_object.is_null() {
            return ::neocogi_com::NC_E_POINTER;
        }
        let state = match unsafe {
            ::neocogi_com::NcCopyFromBufferOffset::<
                u8,
            >::__nc_from_raw_parts(__nc_state_offset, __nc_state_ptr, __nc_state_len)
        } {
            Ok(value) => value,
            Err(status) => return status.normalize_error(),
        };
        let children = match unsafe {
            ::neocogi_com::ComRef::<
                IObjectChildrenAbi,
                ::neocogi_com::ThreadLocal,
            >::__nc_from_raw_borrowed_checked(children)
        } {
            Ok(value) => value,
            Err(status) => return status,
        };
        let mut __nc_dispatch_value = match unsafe {
            <T as ::neocogi_com::ComAbiObject>::borrow_abi_ref(__nc_dispatch_object)
        } {
            Ok(value) => value,
            Err(status) => return status,
        };
        let result = <T as IPersistenceFactory>::restore(
            &*__nc_dispatch_value,
            version,
            state,
            children,
        );
        drop(__nc_dispatch_value);
        match result {
            Ok(value) => {
                unsafe {
                    *__nc_out = value.into_raw();
                }
                ::neocogi_com::NC_OK
            }
            Err(status) => status.normalize_error(),
        }
    }
}
