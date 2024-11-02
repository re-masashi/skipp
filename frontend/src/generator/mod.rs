mod expression;
mod function;
mod program;
mod class;

use crate::c_str;
use crate::parser::{AstNode, NodePosition};
use crate::Result;
use libc::c_char;
use llvm_sys::analysis::LLVMVerifierFailureAction;
use llvm_sys::prelude::{LLVMBuilderRef, LLVMContextRef, LLVMModuleRef, LLVMTypeRef, LLVMValueRef};
use llvm_sys::target_machine::{
    LLVMCodeGenFileType, LLVMCodeGenOptLevel, LLVMCodeModel, LLVMRelocMode, LLVMTarget,
};
use llvm_sys::{analysis, core, target, target_machine};
use log::{debug, error, info, trace, warn};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ffi::CStr;
use std::process::Command;
use std::ptr;

/// Generates LLVM IR based on the AST.
pub struct Generator {
    /// The root of the AST.
    program: Vec<(AstNode, NodePosition)>,

    /// LLVM Context.
    context: LLVMContextRef,
    /// LLVM Module.
    module: LLVMModuleRef,
    /// LLVM Builder.
    builder: LLVMBuilderRef,

    /// LLVM variable map.
    local_vars: RefCell<HashMap<String, (LLVMValueRef, LLVMTypeRef)>>,
    /// Variables in the current scope
    scope_var_names: RefCell<Vec<Vec<String>>>,
    /// current function
    current_fn: RefCell<Option<LLVMValueRef>>,
    /// number of if statements in the current function
    if_count: RefCell<i32>,
    /// struct name-type mapping
    structs: RefCell<HashMap<String, (LLVMTypeRef, HashMap<String, String>)>>,
    classes: RefCell<HashSet<String>>,
    /*
    {
        "struct1": (0xb1a4b1a4, {
            "name": "string",
            "age": "i8",
        })
    }
    */
}

impl Generator {
    /// Create a new generator from a [`Program`].
    ///
    /// [`Program`]: ../parser/program/struct.Program.html
    ///
    /// # Arguments
    /// * `program` - The root of the AST.
    /// * `name` - The name of the module to be created.
    pub unsafe fn new(program: Vec<(AstNode, NodePosition)>, name: &str) -> Self {
        let context = core::LLVMContextCreate();
        Generator {
            program,
            context,
            module: core::LLVMModuleCreateWithNameInContext(c_str!(name), context),
            builder: core::LLVMCreateBuilderInContext(context),
            local_vars: RefCell::new(HashMap::new()),
            scope_var_names: RefCell::new(Vec::new()),
            current_fn: RefCell::new(None),
            if_count: RefCell::new(0),
            structs: RefCell::new(HashMap::new()),
            classes: RefCell::new(HashSet::new()),
        }
    }

    pub unsafe fn init(&self){
        // let struct_lltype = core::LLVMStructCreateNamed(
        //     self.context,
        //     c_str!("Person")
        // );
        // core::LLVMStructSetBody(
        //     struct_lltype,
        //     vec![self.i32_type(), self.bool_type()].as_mut_ptr(),
        //     2,
        //     0,
        // );
        // let mut field_data = HashMap::new();
        // field_data.insert(String::from("age"), String::from("i32"));
        // field_data.insert(String::from("alive"), String::from("bool"));        
        // (*self.structs.borrow_mut()).insert(String::from("Person"), (struct_lltype.clone(), field_data));
        let llvm_fn = core::LLVMAddFunction(
            self.module,
            c_str!("GC_malloc"),
            core::LLVMFunctionType(
                self.void_type(),
                [self.i64_type()].as_mut_ptr(),
                1, // name.len() == type_.len()
                0,
            ),
        );
        // let struct_llval = core::LLVMConstStructInContext(
        //     self.context,
        //     vec![
        //         core::LLVMConstInt(self.i32_type(), 402, false as i32),
        //         core::LLVMConstInt(self.bool_type(), 0, false as i32),
        //     ].as_mut_ptr(),
        //     2,
        //     0
        // );
        // let mut local_vars_mut = self.local_vars.borrow_mut();

        // let var = core::LLVMBuildAlloca(
        //     self.builder, 
        //     core::LLVMGetTypeByName2(self.context, c_str!("Person")),
        //     c_str!("")
        // );

        // let name = "person1";

        // info!("Adding `{}` to local vars", name);
        // local_vars_mut.insert(String::from(name), (var, struct_lltype));
        // self.scope_var_names
        //     .borrow_mut()
        //     .last_mut()
        //     .unwrap()
        //     .push(String::from(name));

        // drop(local_vars_mut);

        // core::LLVMBuildStore(self.builder, 
        //     struct_llval, 
        //     match self.local_vars.borrow().get(name) {
        //         Some((v, _))=>*v,
        //         None=>panic!("No such variable")
        //     }
        // );
    }

    /// Generate the LLVM IR from the module.
    pub unsafe fn generate(&self) -> Result<()> {
        self.gen_program(&self.program)?;
        debug!("Successfully generated program");
        debug!("{:?}", self.structs.borrow());
        Ok(())
    }

    pub unsafe fn optimize(&self) {
        use llvm_sys::core::*;
        use llvm_sys::transforms::pass_manager_builder::*;
        use llvm_sys::transforms::instcombine::LLVMAddInstructionCombiningPass;
        use llvm_sys::transforms::vectorize::LLVMAddLoopVectorizePass;
        use llvm_sys::transforms::scalar::*;

        // Per clang and rustc, we want to use both kinds.
        let fpm = LLVMCreateFunctionPassManagerForModule(self.module);
        let mpm = LLVMCreatePassManager();

        // Populate the pass managers with passes
        let pmb = LLVMPassManagerBuilderCreate();
        LLVMPassManagerBuilderSetOptLevel(pmb, 2);
        LLVMAddInstructionCombiningPass(fpm);
        LLVMAddLoopVectorizePass(fpm);
        LLVMAddLoopUnrollPass(fpm);
        LLVMAddLoopRotatePass(fpm);
        // Magic threshold from Clang for -O2
        LLVMPassManagerBuilderUseInlinerWithThreshold(pmb, 225);
        LLVMPassManagerBuilderPopulateModulePassManager(pmb, mpm);
        LLVMPassManagerBuilderPopulateFunctionPassManager(pmb, fpm);
        LLVMPassManagerBuilderDispose(pmb);

        // Iterate over functions, running the FPM over each
        LLVMInitializeFunctionPassManager(fpm);
        let mut func = LLVMGetFirstFunction(self.module);
        while func != ptr::null_mut() {
            LLVMRunFunctionPassManager(fpm, func);
            func = LLVMGetNextFunction(func);
        }
        LLVMFinalizeFunctionPassManager(fpm);

        // Run the MPM over the module
        LLVMRunPassManager(mpm, self.module);

        // Clean up managers
        LLVMDisposePassManager(fpm);
        LLVMDisposePassManager(mpm);

    }

    /// Verify LLVM IR.
    pub unsafe fn verify(&self) -> Result<()> {
        let mut error = ptr::null_mut::<c_char>();
        analysis::LLVMVerifyModule(
            self.module,
            LLVMVerifierFailureAction::LLVMReturnStatusAction,
            &mut error,
        );
        if !error.is_null() {
            let error = CStr::from_ptr(error).to_str().unwrap().to_string();
            if !error.is_empty() {
                return Err(error);
            }
        }
        debug!("Successfully verified module");
        Ok(())
    }

    /// Dump LLVM IR to stdout.
    pub unsafe fn generate_ir(&self, output: &str) -> Result<()> {
        let mut error = ptr::null_mut::<c_char>();
        core::LLVMPrintModuleToFile(self.module, c_str!(output), &mut error);
        if !error.is_null() {
            let error = CStr::from_ptr(error).to_str().unwrap().to_string();
            if !error.is_empty() {
                return Err(error);
            }
        }
        Ok(())
    }

    /// Generate an object file from the LLVM IR.
    ///
    /// # Arguments
    /// * `optimization` - Optimization level (0-3).
    /// * `output` - Output file path.
    pub unsafe fn generate_object_file(&self, optimization: u32, output: &str) -> Result<()> {
        let target_triple = target_machine::LLVMGetDefaultTargetTriple();

        info!(
            "Target: {}",
            CStr::from_ptr(target_triple).to_str().unwrap()
        );

        target::LLVM_InitializeAllTargetInfos();
        target::LLVM_InitializeAllTargets();
        target::LLVM_InitializeAllTargetMCs();
        target::LLVM_InitializeAllAsmParsers();
        target::LLVM_InitializeAllAsmPrinters();
        trace!("Successfully initialized all LLVM targets");

        let mut target = ptr::null_mut::<LLVMTarget>();
        let mut error = ptr::null_mut::<c_char>();
        target_machine::LLVMGetTargetFromTriple(target_triple, &mut target, &mut error);
        if !error.is_null() {
            let error = CStr::from_ptr(error).to_str().unwrap().to_string();
            if !error.is_empty() {
                return Err(error);
            }
        }

        let optimization_level = match optimization {
            0 => LLVMCodeGenOptLevel::LLVMCodeGenLevelNone,
            1 => LLVMCodeGenOptLevel::LLVMCodeGenLevelLess,
            2 => LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault,
            3 => LLVMCodeGenOptLevel::LLVMCodeGenLevelAggressive,
            _ => {
                warn!("Invalid optimization level, defaulting to 2");
                LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault
            }
        };
        info!("Optimization level: {}", optimization);

        let target_machine = target_machine::LLVMCreateTargetMachine(
            target,
            target_triple,
            c_str!("generic"),
            c_str!(""),
            optimization_level,
            LLVMRelocMode::LLVMRelocDefault, // TODO is this right?
            LLVMCodeModel::LLVMCodeModelDefault, // TODO is this right?
        );
        trace!("Successfully created target machine");

        let mut target = ptr::null_mut::<c_char>();
        target_machine::LLVMTargetMachineEmitToFile(
            target_machine,
            self.module,
            c_str!(output) as *mut _,
            LLVMCodeGenFileType::LLVMObjectFile,
            &mut target,
        );
        if !target.is_null() {
            let error = CStr::from_ptr(error).to_str().unwrap();
            error!("{}", error);
        };
        trace!("Successfully emitted to file");
        Ok(())
    }

    /// Generates an executable from the object file by calling gcc.
    ///
    /// # Arguments
    /// * `object_file` - Path to the object file.
    /// * `output` - Path to the executable.
    pub fn generate_executable(&self, object_file: &str, output: &str) -> Result<()> {
        // TODO is there a better way to do this?
        match Command::new("g++")
            .args(&[object_file, "std.cc", "-o", output])
            .spawn()
        {
            Ok(_) => {
                debug!("Successfully generated executable: {}", output);
                Ok(())
            }
            Err(e) => Err(format!("Unable to link object file:\n{}", e)),
        }
    }

    fn no_terminator(&self) -> bool {
        let block = unsafe {core::LLVMGetInsertBlock(self.builder)};
        let terminator = unsafe {core::LLVMGetBasicBlockTerminator(block)};
        return terminator.is_null();
    }

    /// Get LLVM i32 type in context.
    #[inline]
    fn i32_type(&self) -> LLVMTypeRef {
        unsafe { core::LLVMInt32TypeInContext(self.context) }
    }

    /// Get LLVM i64 type in context.
    #[inline]
    fn i64_type(&self) -> LLVMTypeRef {
        unsafe { core::LLVMInt64TypeInContext(self.context) }
    }

    /// Get LLVM i1 type in context.
    #[inline]
    fn bool_type(&self) -> LLVMTypeRef {
        unsafe { core::LLVMInt1TypeInContext(self.context) }
    }

    /// Get LLVM void type in context
    #[inline]
    fn void_type(&self) -> LLVMTypeRef {
        unsafe { core::LLVMVoidTypeInContext(self.context) }
    }

    /// Get LLVM string type in context
    #[inline]
    fn str_type(&self, length: u32) -> LLVMTypeRef {
        unsafe { core::LLVMArrayType(core::LLVMInt8TypeInContext(self.context), length) }
    }

    #[inline]
    fn pstr_type(&self) -> LLVMTypeRef {
        unsafe { core::LLVMPointerType(core::LLVMInt8TypeInContext(self.context), 8) }
    }

    #[inline]
    fn parr_type(&self) -> LLVMTypeRef {
        unsafe { core::LLVMPointerType(core::LLVMInt32TypeInContext(self.context), 8) }
    }

    #[inline]
    fn array_type(&self, length: u32, type_: LLVMTypeRef) -> LLVMTypeRef {
        unsafe { core::LLVMArrayType(type_, length) }
    }

    #[inline]
    fn struct_type(&self, name: String) -> LLVMTypeRef {
        match self.structs.borrow().get(&name) {
            Some((type_, _)) => type_.clone(),
            None => panic!("no such struct"),
        }
    }

    fn str_to_type(&self, ty: String) -> LLVMTypeRef{
        match ty.as_str(){
            "i32" => self.i32_type(),
            "i64" => self.i32_type(),
            "bool"=>self.bool_type(),
            "void"=>self.void_type(), 
            // "string"=> 
            "str"=>self.pstr_type(), 
            "intarr"=>self.parr_type(),
            x=>{
                print!("{:?}",x);
                match (self.structs.borrow()).get(x) {
                    Some((ty,type_map)) => ty.clone(),
                    None => panic!("No such struct {} found!", x),
                }
            }
        }
    }
}

impl Drop for Generator {
    fn drop(&mut self) {
        debug!("Cleaning up generator");
        unsafe {
            core::LLVMDisposeBuilder(self.builder);
            core::LLVMDisposeModule(self.module);
            core::LLVMContextDispose(self.context);
        }
    }
}

/// Convert a `&str` into `*const libc::c_char`
#[macro_export]
macro_rules! c_str {
    ($s:expr) => {
        format!("{}\0", $s).as_ptr() as *const libc::c_char
    };
}