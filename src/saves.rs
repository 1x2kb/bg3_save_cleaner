use crate::save_information::SaveInformation;

#[derive(Debug, PartialEq, Default, Clone)]
pub struct Saves {
    pub quick_saves: Vec<SaveInformation>,
    pub auto_saves: Vec<SaveInformation>,
}
impl Saves {
    pub fn new() -> Self {
        Saves {
            quick_saves: Vec::new(),
            auto_saves: Vec::new(),
        }
    }
}
