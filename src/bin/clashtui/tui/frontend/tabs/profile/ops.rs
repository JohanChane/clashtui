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
    /// test the profile content
    ///
    /// > I don't really know what `geodata_mode` can do,
    /// > but I'll keep it
    Test(String, bool),
    /// ask for preview
    ///
    /// though this is asked by tab, but it will be handled at frontend
    Preview(String),
    /// ask for edit
    ///
    /// though this is asked by tab, but it will be handled at frontend
    Edit(String),
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
