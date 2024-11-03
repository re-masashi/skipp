use crate::c_str;
use crate::generator::Generator;
use crate::parser::ExprValue;
use crate::lexer::tokens::TokenType;
use crate::Result;
use llvm_sys::core;
use llvm_sys::prelude::{LLVMValueRef, LLVMBasicBlockRef, LLVMTypeRef};
use llvm_sys::LLVMIntPredicate;
use log::{trace, info};

use std::collections::HashMap;

impl Generator {
    pub unsafe fn gen_expression(&self, expression: &ExprValue) -> Result<(LLVMValueRef, LLVMTypeRef)> {
        trace!("Generating expression");
        match expression {
            ExprValue::Integer(i) => {
                return Ok((
                    core::LLVMConstInt(self.i32_type(), *i as u64, false as i32),
                    self.i32_type()
                ));
            }
            ExprValue::Do(expressions) => {
                let mut ret_val = Ok((core::LLVMConstInt(self.i32_type(), 0 as u64, false as i32), self.i32_type()));
                for expression in expressions {
                    ret_val = self.gen_expression(expression);
                }
                return ret_val
            }
            ExprValue::Str(s) => {
                return Ok((core::LLVMConstString(
                    c_str!(s),
                    s.len() as u32,
                    false as i32,
                ), self.str_type(s.len() as u32)));
            }
            ExprValue::Boolean(b)=>{
                trace!("Boolean literal: {}", *b as u64);
                Ok((core::LLVMConstInt(self.bool_type(), *b as u64, false as i32), self.bool_type()))
            }
            ExprValue::Array(v, t)=>{
               let mut vals=  v.iter().map(|x| self.gen_expression(x).expect("oops").0).collect::<Vec<_>>();
               return Ok((core::LLVMConstArray(
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
                ), self.array_type(vals.len() as u32, match t.as_str() {
                        "i32" => self.i32_type(),
                        "bool"=>self.bool_type(),
                        "void"=>self.void_type(),
                        "str"=>self.pstr_type(),               
                        s if &s[0..3] == "str" => {
                            self.str_type(s[3..].parse::<u32>().unwrap())
                        },
                        _=>todo!()
                    },)))
            }
            ExprValue::UnOp ( op, expression ) => {
                trace!("Generating unary expression");
                match **op {
                    TokenType::Minus => {
                        let (expr, type_) = self.gen_expression(expression)?;
                        Ok((core::LLVMBuildNeg(
                            self.builder,
                            expr,
                            c_str!(""),
                        ), type_))
                    }
                    TokenType::Not => {
                        let (expr, type_) = self.gen_expression(expression)?;
                        Ok((core::LLVMBuildNot(
                            self.builder,
                            expr,
                            c_str!(""),
                        ), type_))
                    }
                    _ => Err("Unidentified unary expression".to_string()),
                }
            }
            ExprValue::Identifier(name) => {
                
                if let Some((var, lltype)) = self.local_vars.borrow().get(name) {
                    trace!("Local variable: {}", name);
                    Ok((core::LLVMBuildLoad2(
                        self.builder,
                        lltype.clone(),
                        *var,
                        c_str!(""),
                    ), lltype.clone()))
                } else {
                    
                    Err(format!("Unresolved variable reference `{}`", name))
                }
            }
            ExprValue::FnCall(name, args) => {

                // if self.classes.borrow().contains(name) {
                //     println!("Class pointer {:?}", core::LLVMGetTypeByName2(self.context, c_str!("$".to_owned()+name)));
                //     let class = core::LLVMGetTypeByName2(self.context, c_str!("$".to_owned()+name));
                //     let dummy_ptr = core::LLVMBuildStructGEP2(
                //         self.builder,
                //         class,
                //         core::LLVMConstNull(class),
                //         0,
                //         c_str!("objptr"),
                //     );
                //     // println!("obj size {:?}", object_size);
                //     println!("GC_malloc {:?}", core::LLVMGetNamedFunction(self.module, c_str!("GC_malloc")));
                //     // NOT LINKED! 
                //     let obj_void_ptr = core::LLVMBuildCall2(
                //         self.builder,
                //         class,
                //         core::LLVMGetNamedFunction(self.module, c_str!("GC_malloc")),
                //         [core::LLVMConstInt(self.i32_type(), 4, false as i32)].as_mut_ptr(),
                //         1,
                //         c_str!("")
                //     );
                //     println!("object {:?}", obj_void_ptr);
                //     let obj = core::LLVMBuildPointerCast(
                //         self.builder,
                //         obj_void_ptr,
                //         class,
                //         c_str!(""),
                //     );
                //     let vtable_name = "$_VTable".to_owned()+name;
                //     let vtable_field = core::LLVMBuildStructGEP2(
                //         self.builder,
                //         class,
                //         obj,
                //         0,
                //         c_str!("obj"),
                //     );
                //     println!("vtable_field (obj pointer) {:?}", vtable_field);
                //     let vtable = core::LLVMGetNamedGlobal(self.module, c_str!(vtable_name));
                //     // println!("VTable {:?}", vtable);
                //     if vtable==std::ptr::null_mut() {
                //         panic!("No Vtable found for {:}", vtable_name);
                //     }
                //     core::LLVMBuildStore(self.builder, vtable, vtable_field);
                //     println!("Classes todo!");
                //     return Ok(vtable)
                // }

                match self.structs.borrow().get(name) {
                    Some((t,v))=>{
                        if args.len()!=v.len() {
                            panic!("NO! INCORRECT NUMBER OF STRUCT PARAMS");
                        }
                        let mut vals = vec![];
                        for arg in args {
                            vals.push(self.gen_expression(arg)?.0);
                        }
                        let var = core::LLVMBuildAlloca(
                            self.builder, 
                            core::LLVMGetTypeByName2(self.context, c_str!("$struct$".to_owned()+name)),
                            c_str!("")
                        );
                        let struct_init = core::LLVMBuildStore(self.builder, 
                            var, 
                            core::LLVMConstNamedStruct(t.clone(), vals.as_mut_ptr(), vals.len() as u32)
                        );
                        println!("Struct type pointer {:?}", core::LLVMGetTypeByName2(self.context, c_str!("$struct$".to_owned()+name)));
                        return Ok((struct_init, t.clone()));
                        // panic!("aaaaaaaaaaa");
                    }
                    None=>{}
                    // return Ok(vtable)
                }

                // if name.as_str()=="init_struct" {
                //     self.init();
                //     println!("INIT!");
                //     return Ok(core::LLVMConstInt(self.i32_type(), 0, false as i32));
                // }
                
                let mut llvm_args: Vec<LLVMValueRef> = Vec::new();
                for arg in args {
                    llvm_args.push(self.gen_expression(arg)?.0);
                }

                let function = core::LLVMGetNamedFunction(self.module, c_str!(name));
                if function.is_null() {
                    return Err(format!("Function `{}` doesn't exist", name));
                }
                Ok((core::LLVMBuildCall2(
                    self.builder,
                    llvm_sys::core::LLVMGlobalGetValueType(function),
                    function,
                    llvm_args.as_mut_ptr(),
                    args.len() as u32,
                    c_str!(""),
                ), llvm_sys::core::LLVMGlobalGetValueType(function)))
            }
            ExprValue::Return (expr) => {
                
                let (val, type_) = self.gen_expression(expr)?;
                core::LLVMBuildRet(self.builder, val);
                Ok((val, type_))
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
                    
                    let (val, type_) = self.gen_expression(v)?;
                    assert_eq!(type_, lltype);
                    return Ok((core::LLVMBuildStore(self.builder, 
                        val, 
                        match self.local_vars.borrow().get(name) {
                            Some((v, _))=>*v,
                            None=>panic!("No such variable")
                        }
                    ), type_))
                }
                Ok((var, lltype))
            }
            ExprValue::Assign {name, value} =>{
                let (expr, type_) = self.gen_expression(value)?;
                Ok((core::LLVMBuildStore(self.builder, 
                    expr, 
                    match self.local_vars.borrow().get(name) {
                        Some((v, _))=>*v,
                        None=>panic!("No such variable")
                    }
                ), type_))
            }
            ExprValue::BinOp(lhs, op, rhs) =>{

                let (l, type_) = self.gen_expression(lhs)?;

                if let TokenType::Dot=**op{
                    let mut struct_: HashMap<String, (String, i32)> = HashMap::new();
                    let mut elem_type = " ".to_string();

                    for (name, (ty, struct__)) in self.structs.borrow().iter() {
                        if *ty==type_ {
                            struct_ = struct__.clone();
                            break;
                        }
                    }
                    // todo!();
                    match **rhs {
                        ExprValue::Integer(i)=>{
                            // Ok(core::LLVMBuildGEP2(
                            //     self.builder, 
                            //     type_,
                            //     l,
                            //     [core::LLVMConstInt(self.i32_type(), 0, 0)].as_mut_ptr(),
                            //     i,
                            //     c_str!("struct_gep")
                            // ), type_);
                            todo!()
                        }, //(i as usize).into().unwrap(),
                        ExprValue::Identifier(ref i)=>{
                            println!("GEP");
                            match struct_.get(i) {
                                Some((type__, index)) => {
                                    // *index as u32
                                    println!("GEP 2 {} {}({})", type__, i, index);
                                    let x =  (core::LLVMBuildStructGEP2(
                                        self.builder, 
                                        type_,
                                        l,
                                        // [
                                        //     core::LLVMConstInt(self.i32_type(), 0, 0),
                                        // ].as_mut_ptr(),
                                        *index as u32,
                                        c_str!("struct_gep")
                                    ), self.str_to_type(type__.to_string()));
                                    println!("{:?} {:?}", x.0, x.1);
                                    return Ok(x)
                                },
                                None => {
                                    panic!("invalid param accessed");
                                },
                            }
                        },
                        _=>{
                            println!("{:?}", rhs);
                            panic!("aa");
                        },
                    }
                    // return Ok((core::LLVMConstInt(self.i32_type(), 21, 0), self.i32_type()))
                }

                let (l, type_l) = self.gen_expression(lhs)?;
                let (r, _type_r) = self.gen_expression(rhs)?;

                // todo: handle if type_l and type_r are different
                // for now, type of the entire expression is type_l

                if let ExprValue::Str(_) = **lhs {
                    todo!()
                }

                if let ExprValue::Str(_) = **rhs {
                    todo!()
                }

                match **op {
                    TokenType::Plus => Ok((core::LLVMBuildAdd(self.builder, l, r, c_str!("")), type_l)),
                    TokenType::Minus => Ok((core::LLVMBuildSub(self.builder, l, r, c_str!("")), type_l)),
                    TokenType::Mul => Ok((core::LLVMBuildMul(self.builder, l, r, c_str!("")), type_l)),
                    TokenType::Div => Ok((core::LLVMBuildSDiv(self.builder, l, r, c_str!("")), type_l)),
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
                        Ok((cmp_i32, self.bool_type()))
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
                
                let (if_expr, if_type) = self.gen_expression(if_).unwrap();

                core::LLVMBuildBr(self.builder, end);
                
                let else_bb = core::LLVMAppendBasicBlock(current_fn, 
                    c_str!(format!("else.{}", *self.if_count.borrow()))
                );
                core::LLVMPositionBuilderAtEnd(self.builder, else_bb);

                let (else_expr, else_type) = self.gen_expression(else_).unwrap();

                core::LLVMBuildBr(self.builder, end);

                core::LLVMPositionBuilderAtEnd(self.builder, entry);
                let (cond_llvm, _) = self.gen_expression(cond)?;
                core::LLVMBuildCondBr(self.builder, cond_llvm, if_bb, else_bb);
                core::LLVMPositionBuilderAtEnd(self.builder, end);

                // let if_val = if if_exprs.len() == 0{
                //     cond_llvm
                // }else{
                //     if_expr
                // };

                // let else_val = if else_exprs.len() == 0{
                //     cond_llvm
                // }else{
                //     match else_exprs.last(){
                //         Some(v)=>v.clone(),
                //         _=>unreachable!()
                //     }
                // };

                let phi = core::LLVMBuildPhi(
                    self.builder, 
                    match type_.as_str(){
                        "i32" => self.i32_type(),
                        "bool"=>self.bool_type(),
                        "void"=>self.void_type(),                    
                        s if &s[0..3] == "str" => {
                            
                            self.str_type(s[3..].parse::<u32>().unwrap())
                        },
                        x=>{
                            print!("{:?}",x);
                            todo!()
                        }
                    },
                    c_str!("fie")
                );

                let (mut values, mut basic_blocks): (Vec<LLVMValueRef>, Vec<LLVMBasicBlockRef>) = 
                    (vec![if_expr, else_expr],
                    vec![end, end]);

                core::LLVMAddIncoming(
                    phi,
                    values.as_mut_ptr(),
                    basic_blocks.as_mut_ptr(),
                    2,
                );

                Ok((if_expr, if_type))
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
                let (cond_llvm, type_) = self.gen_expression(cond)?;

                core::LLVMBuildCondBr(self.builder, cond_llvm, body, end);
                core::LLVMPositionBuilderAtEnd(self.builder, body);

                self.gen_expression(&*exprs);
                
                todo!()
            }
            x=>{
                
                todo!()
            }
        }
    }
}