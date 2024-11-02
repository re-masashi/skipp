use crate::c_str;
use crate::generator::Generator;
use crate::parser::Class;
use crate::Result;
use llvm_sys::core;
use llvm_sys::prelude::{LLVMValueRef, LLVMTypeRef};
use llvm_sys::LLVMIntPredicate;
use log::{trace, info};
use std::collections::HashMap;

impl Generator {
    pub unsafe fn gen_class(&self, class: &Class) {
        trace!("Generating class");
        let struct_lltype = core::LLVMStructCreateNamed(
            self.context,
            c_str!("$".to_owned()+&class.name)
        );
        let struct_lltype_vtable = core::LLVMStructCreateNamed(
            self.context,
            c_str!("$_VTable".to_owned()+&class.name)
        );
        let mut field_data = HashMap::new();
        field_data.insert(String::from("vtableptr"), String::from("$_VTable".to_owned()+&class.name));
        (*self.structs.borrow_mut()).insert(class.name.clone(), (struct_lltype.clone(), field_data));
        core::LLVMStructSetBody(
            struct_lltype,
            vec![struct_lltype_vtable].as_mut_ptr(),
            2,
            0,
        );
        core::LLVMAddGlobal(self.module, struct_lltype_vtable, c_str!("$_VTable".to_owned()+&class.name));
        self.gen_vtable(struct_lltype_vtable, class);
        (*self.classes.borrow_mut()).insert(class.name.clone());
    }

    pub unsafe fn gen_vtable(&self, vtable: LLVMTypeRef, class: &Class) {
        let vtable = core::LLVMGetNamedGlobal(self.module, c_str!("$_VTable".to_owned()+&class.name));
        trace!("Generating vtable");
    }
    
}