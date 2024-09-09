use super::*;
use crate::tui::tabs::profile::TemplateOp;

impl BackEnd {
    pub fn get_all_templates(&self) -> Vec<String> {
        self.inner.get_all_templates()
    }
}

impl BackEnd {
    pub(super) fn handle_template_op(&self, op: TemplateOp) -> CallBack {
        match op {
            TemplateOp::GetALL => CallBack::TemplateInit(self.get_all_templates()),
            TemplateOp::Add(_) => todo!(),
            TemplateOp::Remove(_) => todo!(),
            TemplateOp::Generate(_) => todo!(),
        }
    }
}
