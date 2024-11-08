use crate::c_str;
use crate::generator::Generator;
use crate::parser::Class;
use crate::Result;
use llvm_sys::core;
use llvm_sys::prelude::{LLVMTypeRef, LLVMValueRef};
use llvm_sys::LLVMIntPredicate;
use log::{info, trace};
use std::collections::HashMap;

impl Generator {
    pub unsafe fn gen_class(&self, class: &Class) {
        todo!()
        // trace!("Generating class");
        // let struct_lltype = core::LLVMStructCreateNamed(
        //     self.context,
        //     c_str!("$".to_owned()+&class.name)
        // );
        // let struct_lltype_vtable = core::LLVMStructCreateNamed(
        //     self.context,
        //     c_str!("$_VTable".to_owned()+&class.name)
        // );
        // let mut field_data = HashMap::new();
        // field_data.insert(String::from("vtableptr"), String::from("$_VTable".to_owned()+&class.name));
        // (*self.structs.borrow_mut()).insert(class.name.clone(), (struct_lltype.clone(), field_data));
        // core::LLVMStructSetBody(
        //     struct_lltype,
        //     vec![struct_lltype_vtable].as_mut_ptr(),
        //     2,
        //     0,
        // );
        // core::LLVMAddGlobal(self.module, struct_lltype_vtable, c_str!("$_VTable".to_owned()+&class.name));
        // self.gen_vtable(struct_lltype_vtable, class);
        // (*self.classes.borrow_mut()).insert(class.name.clone());
    }

    pub unsafe fn gen_vtable(&self, vtable: LLVMTypeRef, class: &Class) {
        todo!()
        // let vtable_name = c_str!("$_VTable".to_owned()+&class.name);
        // let vtable = core::LLVMGetTypeByName2(self.context, vtable_name);
        // let mut vtable_methods = vec![];
        // let mut vtable_method_types = vec![];
        // for (method, _) in &class.fns {
        //     vtable_methods.push(core::LLVMGetNamedFunction(self.module, c_str!(method.name)));
        //     vtable_method_types.push(self.str_to_type(method.return_type.clone()));
        // }
        // trace!("Generating vtable");
    }

    pub unsafe fn gen_struct(&self, name: &String, struct_: &HashMap<String, (String, i32)>) {
        let struct_lltype =
            core::LLVMStructCreateNamed(self.context, c_str!("$struct$".to_owned() + &name));
        let mut types = vec![];
        for (key, (value, index)) in struct_.into_iter() {
            types.push(self.str_to_type(value.to_string()));
            println!("{} : {} ({})", key, value, index);
        }
        core::LLVMStructSetBody(struct_lltype, types.as_mut_ptr(), types.len() as u32, 0);
        core::LLVMAddGlobal(
            self.module,
            struct_lltype,
            c_str!("$struct_ty$".to_owned() + &name),
        );
        (*self.structs.borrow_mut()).insert(name.clone(), (struct_lltype, struct_.clone()));
        trace!("Generating struct");
    }
}
