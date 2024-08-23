pub enum BackendOp {
    Profile(ProfileOp),
    #[cfg(feature = "template")]
    Template(TemplateOp),
}

pub enum ProfileOp {
    /// Send after [ProfileOp::Update],[ProfileOp::Add],[ProfileOp::Remove]
    /// and [TemplateOp::Generate] if enable template feature
    GetALL,
    /// go wwithout ask
    Select(String),
    /// go wwithout ask
    Add(String, String),
    /// ask for two option(`Yes/No`)
    Remove(String),
    /// ask for three option(`Yes/No/Auto`)
    ///
    /// Currently, `Auto` is treated as `No` in the BackEnd
    Update(String, Option<bool>),
}
#[cfg(feature = "template")]
pub enum TemplateOp {
    /// Send after [TemplateOp::Add],[TemplateOp::Remove]
    GetALL,
    /// go wwithout ask
    Generate(String),
    /// go wwithout ask
    Add(String),
    /// ask for two option(`Yes/No`)
    Remove(String),
}
