use crate::c_str;
use crate::generator::Generator;
use crate::parser::{Function, ExprValue, External};
use crate::Result;
use llvm_sys::core;
use llvm_sys::prelude::LLVMTypeRef;
use log::{info, trace};

impl Generator {
    pub unsafe fn gen_function(&self, function: &Function) -> Result<()> {
        trace!("Generating function");

        let args = &function.args;
        let mut arg_types: Vec<LLVMTypeRef> = Vec::new();

        for arg in args.type_.iter() {
            arg_types.insert(arg_types.len(),
                self.str_to_type(arg.to_string())
            )
        }

        let return_type = self.str_to_type(function.return_type.clone());

        // self.functions.borrow_mut().insert(function.name.clone(), return_type);

        // All args are i32 for now
        // let mut arg_types = vec![self.i32_type(); args.name.len()];

        // Create function
        let llvm_function = core::LLVMAddFunction(
            self.module,
            c_str!(function.name),
            core::LLVMFunctionType(
                return_type,
                arg_types.as_mut_ptr(),
                args.name.len() as u32,
                0,
            ),
        );

        *self.current_fn.borrow_mut() = Some(llvm_function);
        *self.if_count.borrow_mut() = 0;

        let entry =
            core::LLVMAppendBasicBlockInContext(self.context, llvm_function, c_str!("entry"));

        core::LLVMPositionBuilderAtEnd(self.builder, entry);

        // if self.structs.borrow().len() == 0{
        //     self.init();
        // }

        for (i, arg_name) in args.name.iter().enumerate() {
            // Set arg name in function prototype
            let arg = core::LLVMGetParam(llvm_function, i as u32);
            core::LLVMSetValueName2(arg, c_str!(arg_name), arg_name.len());

            let mut local_vars_mut = self.local_vars.borrow_mut();
            let t = &args.type_[i];
            let lltype = self.str_to_type(t.clone());
            let var = core::LLVMBuildAlloca(self.builder, lltype, c_str!(""));

            if arg_name != "_" {
                local_vars_mut.insert(arg_name.to_string(), (var, lltype));
            }

            core::LLVMBuildStore(self.builder, arg, var);
        }

        self.scope_var_names.borrow_mut().push(Vec::new());
        
        for expr in &function.expressions {
            self.gen_expression(&expr)?;
        }

        if let Some(ExprValue::Return(_)) = &function.expressions.last() {
        }else{
            
            let zero = core::LLVMConstInt(self.i32_type(), 0 as u64, false as i32);
            core::LLVMBuildRet(self.builder, zero);       
        }

        let mut local_vars_mut = self.local_vars.borrow_mut();
        for var in self.scope_var_names.borrow().last().unwrap() {
            
            local_vars_mut.remove(var);
        }
        
        self.scope_var_names.borrow_mut().pop();
        Ok(())
    }

    pub unsafe fn gen_extern(&self, function: &External) -> Result<()> {
        trace!("Generating extern");

        let args = &function.args;

        let name = function.name.clone();

        let mut arg_types: Vec<LLVMTypeRef> = Vec::new();

        for arg in args.type_.iter() {
            arg_types.insert(arg_types.len(),
                self.str_to_type(arg.clone())
            )
        }

        let return_type = self.str_to_type(function.return_type.clone());

        // self.functions.borrow_mut().insert(function.name.clone(), return_type);

        // Create function
        let llvm_fn = core::LLVMAddFunction(
            self.module,
            c_str!(name),
            core::LLVMFunctionType(
                return_type,
                arg_types.as_mut_ptr(),
                args.name.len() as u32, // name.len() == type_.len()
                0,
            ),
        );
        Ok(())
    }
}