use crate::c_str;
use crate::generator::Generator;
use crate::parser::ExprValue;
use crate::lexer::tokens::TokenType;
use crate::Result;
use llvm_sys::core;
use llvm_sys::prelude::{LLVMValueRef, LLVMBasicBlockRef};
use llvm_sys::LLVMIntPredicate;
use log::{trace, info};

impl Generator {
    pub unsafe fn gen_expression(&self, expression: &ExprValue) -> Result<LLVMValueRef> {
        trace!("Generating expression");
        match expression {
            ExprValue::Integer(i) => {
                return Ok(core::LLVMConstInt(self.i32_type(), *i as u64, false as i32));
            }
            ExprValue::Str(s) => {
                return Ok(core::LLVMConstString(
                    c_str!(s),
                    s.len() as u32,
                    false as i32,
                ));
            }
            ExprValue::Boolean(b)=>{
                trace!("Boolean literal: {}", *b as u64);
                Ok(core::LLVMConstInt(self.bool_type(), *b as u64, false as i32))
            }
            ExprValue::Array(v, t)=>{
               let mut vals=  v.iter().map(|x| self.gen_expression(x).expect("oops")).collect::<Vec<_>>();
               return Ok(core::LLVMConstArray(
                    match t.as_str() {
                        "i32" => self.i32_type(),
                        "bool"=>self.bool_type(),
                        "void"=>self.void_type(),
                        "str"=>self.pstr_type(),               
                        s if &s[0..3] == "str" => {
                            self.str_type(s[3..].parse::<u32>().unwrap())
                        },
                        _=>todo!()
                    },
                    vals.as_mut_ptr(),
                    vals.len() as u32
                ))
            }
            ExprValue::UnOp ( op, expression ) => {
                trace!("Generating unary expression");
                match **op {
                    TokenType::Minus => Ok(core::LLVMBuildNeg(
                        self.builder,
                        self.gen_expression(expression)?,
                        c_str!(""),
                    )),
                    TokenType::Not => Ok(core::LLVMBuildNot(
                        self.builder,
                        self.gen_expression(expression)?,
                        c_str!(""),
                    )),
                    _ => Err("Misidentified unary expression".to_string()),
                }
            }
            ExprValue::Identifier(name) => {
                
                if let Some((var, lltype)) = self.local_vars.borrow().get(name) {
                    trace!("Local variable: {}", name);
                    Ok(core::LLVMBuildLoad2(
                        self.builder,
                        lltype.clone(),
                        *var,
                        c_str!(""),
                    ))
                } else {
                    
                    Err(format!("Unresolved variable reference `{}`", name))
                }
            }
            ExprValue::FnCall(name, args) => {

                if self.classes.borrow().contains(name) {
                    println!("Class pointer {:?}", core::LLVMGetTypeByName2(self.context, c_str!("$".to_owned()+name)));
                    let class = core::LLVMGetTypeByName2(self.context, c_str!("$".to_owned()+name));
                    let dummy_ptr = core::LLVMBuildStructGEP2(
                        self.builder,
                        class,
                        core::LLVMConstNull(class),
                        0,
                        c_str!("objptr"),
                    );
                    let object_size = core::LLVMBuildPointerCast(
                        self.builder,
                        dummy_ptr,
                        self.i64_type(),
                        c_str!(""),
                    );
                    println!("obj size {:?}", object_size);
                    println!("GC_malloc {:?}", core::LLVMGetNamedFunction(self.module, c_str!("GC_malloc")));
                    // NOT LINKED! 
                    let obj_void_ptr = core::LLVMBuildCall2(
                        self.builder,
                        class,
                        core::LLVMGetNamedFunction(self.module, c_str!("GC_malloc")),
                        [object_size].as_mut_ptr(),
                        1,
                        c_str!("")
                    );
                    println!("object {:?}", obj_void_ptr);
                    let obj = core::LLVMBuildPointerCast(
                        self.builder,
                        obj_void_ptr,
                        class,
                        c_str!(""),
                    );
                    let vtable_name = "$_VTable".to_owned()+name;
                    let vtable_field = core::LLVMBuildStructGEP2(
                        self.builder,
                        class,
                        obj,
                        0,
                        c_str!("obj"),
                    );
                    println!("vtable_field (obj pointer) {:?}", vtable_field);
                    let vtable = core::LLVMGetNamedGlobal(self.module, c_str!(vtable_name));
                    println!("VTable {:?}", vtable);
                    if vtable==std::ptr::null_mut() {
                        panic!("No Vtable found for {:}", vtable_name);
                    }
                    core::LLVMBuildStore(self.builder, vtable, vtable_field);
                    println!("Classes todo!");
                    return Ok(vtable)
                }

                // if name.as_str()=="init_struct" {
                //     self.init();
                //     println!("INIT!");
                //     return Ok(core::LLVMConstInt(self.i32_type(), 0, false as i32));
                // }
                
                let mut llvm_args: Vec<LLVMValueRef> = Vec::new();
                for arg in args {
                    llvm_args.push(self.gen_expression(arg)?);
                }

                let function = core::LLVMGetNamedFunction(self.module, c_str!(name));
                if function.is_null() {
                    return Err(format!("Function `{}` doesn't exist", name));
                }
                Ok(core::LLVMBuildCall2(
                    self.builder,
                    llvm_sys::core::LLVMGlobalGetValueType(function),
                    function,
                    llvm_args.as_mut_ptr(),
                    args.len() as u32,
                    c_str!(""),
                ))
            }
            ExprValue::Return (expr) => {
                
                let val = self.gen_expression(expr)?;
                core::LLVMBuildRet(self.builder, val);
                Ok(val)
            }
            ExprValue::VarDecl { name, type_, value } => {
                trace!("Generating variable declaration {}", name);
                let mut local_vars_mut = self.local_vars.borrow_mut();

                if local_vars_mut.contains_key(name) {
                    return Err(format!("Variable `{}` already exists", name));
                }

                let lltype = self.str_to_type(type_.to_string());

                let var = core::LLVMBuildAlloca(
                    self.builder, 
                    lltype,
                    c_str!("")
                );
                info!("Adding `{}` to local vars", name);
                local_vars_mut.insert(String::from(name), (var, lltype));
                self.scope_var_names
                    .borrow_mut()
                    .last_mut()
                    .unwrap()
                    .push(String::from(name));

                drop(local_vars_mut);

                if let Some(v) = value {
                    
                    let val = self.gen_expression(v)?;
                    return Ok(core::LLVMBuildStore(self.builder, 
                        val, 
                        match self.local_vars.borrow().get(name) {
                            Some((v, _))=>*v,
                            None=>panic!("No such variable")
                        }
                    ))
                }
                Ok(var)
            }
            ExprValue::Assign {name, value} =>{
                Ok(core::LLVMBuildStore(self.builder, 
                    self.gen_expression(value)?, 
                    match self.local_vars.borrow().get(name) {
                        Some((v, _))=>*v,
                        None=>panic!("No such variable")
                    }
                ))
            }
            ExprValue::BinOp(lhs, op, rhs) =>{
                let l = self.gen_expression(lhs)?;
                let r = self.gen_expression(rhs)?;

                if let ExprValue::Str(_) = **lhs {
                    todo!()
                }

                if let ExprValue::Str(_) = **rhs {
                    todo!()
                }

                match **op {
                    TokenType::Plus => Ok(core::LLVMBuildAdd(self.builder, l, r, c_str!(""))),
                    TokenType::Minus => Ok(core::LLVMBuildSub(self.builder, l, r, c_str!(""))),
                    TokenType::Mul => Ok(core::LLVMBuildMul(self.builder, l, r, c_str!(""))),
                    TokenType::Div => Ok(core::LLVMBuildSDiv(self.builder, l, r, c_str!(""))),
                    TokenType::Equal | TokenType::NotEq | TokenType::Less | TokenType::Greater | TokenType::LessEq | TokenType::GreaterEq => {
                        let cmp = {
                            core::LLVMBuildICmp(
                                self.builder,
                                match **op {
                                    TokenType::Equal => LLVMIntPredicate::LLVMIntEQ,
                                    TokenType::NotEq => LLVMIntPredicate::LLVMIntNE,
                                    TokenType::Less => LLVMIntPredicate::LLVMIntSLT,
                                    TokenType::Greater => LLVMIntPredicate::LLVMIntSGT,
                                    TokenType::LessEq => LLVMIntPredicate::LLVMIntSLE,
                                    TokenType::GreaterEq => LLVMIntPredicate::LLVMIntSGE,
                                    _ => {
                                        return Err(format!(
                                            "Unhandled comparison binary operation",
                                        ))
                                    }
                                },
                                l,
                                r,
                                c_str!(""),
                            )
                        };
                        // Cast i1 to i32
                        let cmp_i32 = {
                            core::LLVMBuildZExt(self.builder, cmp, self.bool_type(), c_str!(""))
                        };
                        Ok(cmp_i32)
                    }
                    _=>todo!()
                }
            }
            ExprValue::IfElse {cond, if_, else_, type_ } => {
                trace!("Generating if else");
                let current_fn = match *self.current_fn.borrow(){
                    Some(s)=>s,
                    _=>unreachable!()
                };
                let entry = core::LLVMGetLastBasicBlock(current_fn);
                if *self.if_count.borrow()>=1 {
                    return Err("Cannot have two if statements in one function".to_string())
                }
                *self.if_count.borrow_mut()+=1;

                let end = core::LLVMAppendBasicBlock(current_fn, 
                        c_str!(format!("end.{}", *self.if_count.borrow()))
                    ); 
                let if_bb = core::LLVMAppendBasicBlock(current_fn, 
                        c_str!(format!("then.{}", *self.if_count.borrow()))
                    );
                core::LLVMPositionBuilderAtEnd(self.builder, if_bb);
                
                let mut if_exprs = vec![];
                for expr in if_{
                    if_exprs.insert(if_exprs.len(), self.gen_expression(&*expr)?);
                }
                core::LLVMBuildBr(self.builder, end);
                
                let else_bb = core::LLVMAppendBasicBlock(current_fn, 
                    c_str!(format!("else.{}", *self.if_count.borrow()))
                );
                core::LLVMPositionBuilderAtEnd(self.builder, else_bb);

                let mut else_exprs = vec![];
                for expr in else_{
                    else_exprs.insert(else_exprs.len(), self.gen_expression(&*expr)?);
                }
                core::LLVMBuildBr(self.builder, end);

                core::LLVMPositionBuilderAtEnd(self.builder, entry);
                let cond_llvm = self.gen_expression(cond)?;
                core::LLVMBuildCondBr(self.builder, cond_llvm, if_bb, else_bb);
                core::LLVMPositionBuilderAtEnd(self.builder, end);

                let if_val = if if_exprs.len() == 0{
                    cond_llvm
                }else{
                    match if_exprs.last(){
                        Some(v)=>v.clone(),
                        _=>unreachable!()
                    }
                };

                let else_val = if else_exprs.len() == 0{
                    cond_llvm
                }else{
                    match else_exprs.last(){
                        Some(v)=>v.clone(),
                        _=>unreachable!()
                    }
                };

                // let phi = core::LLVMBuildPhi(
                //     self.builder, 
                //     match type_.as_str(){
                //         "i32" => self.i32_type(),
                //         "bool"=>self.bool_type(),
                //         "void"=>self.void_type(),                    
                //         s if &s[0..3] == "str" => {
                            
                //             self.str_type(s[3..].parse::<u32>().unwrap())
                //         },
                //         x=>{
                //             print!("{:?}",x);
                //             todo!()
                //         }
                //     },
                //     c_str!("fie")
                // );

                // let (mut values, mut basic_blocks): (Vec<LLVMValueRef>, Vec<LLVMBasicBlockRef>) = 
                //     (vec![if_val, else_val],
                //     vec![end, end]);

                // core::LLVMAddIncoming(
                //     phi,
                //     values.as_mut_ptr(),
                //     basic_blocks.as_mut_ptr(),
                //     2,
                // );

                Ok(if if_exprs.len() == 0{
                    cond_llvm
                }else{
                    match if_exprs.last(){
                        Some(v)=>v.clone(),
                        _=>unreachable!()
                    }
                })
            }
            ExprValue::While(cond, exprs)=>{
                let current_fn = match *self.current_fn.borrow(){
                    Some(s)=>s,
                    _=>unreachable!()
                };
                let entry = core::LLVMGetLastBasicBlock(current_fn);
                let end = core::LLVMAppendBasicBlock(current_fn, 
                        c_str!("while.end")
                    ); 
                let body = core::LLVMAppendBasicBlock(current_fn, 
                        c_str!("while.body")
                    );
                let cond_llvm = self.gen_expression(cond)?;

                core::LLVMBuildCondBr(self.builder, cond_llvm, body, end);
                core::LLVMPositionBuilderAtEnd(self.builder, body);

                for expr in exprs{
                    self.gen_expression(&*expr)?;
                }
                todo!()
            }
            x=>{
                
                todo!()
            }
        }
    }
}