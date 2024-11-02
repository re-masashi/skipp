use crate::parser::{AstNode, NodePosition, Function};
use crate::generator::Generator;
use crate::Result;
use log::trace;

impl Generator {
    pub unsafe fn gen_program(&self, program: &Vec<(AstNode, NodePosition)>) -> Result<()> {
        trace!("Generating program");
        for (node, pos) in program {
            self.local_vars.borrow_mut().clear();
            match node{
                AstNode::FunctionDef(f)=> {self.gen_function(f)?;},
                AstNode::Class(c)=>{self.gen_class(c);},
                AstNode::Expression(e)=>{self.gen_expression(e)?;},
                AstNode::Extern(e)=>{self.gen_extern(e)?;},
                AstNode::Struct(n, s)=>{
                    self.gen_struct(n, s);
                }
            }
            // self.gen_function(&function)?;
        }
        Ok(())
    }
}