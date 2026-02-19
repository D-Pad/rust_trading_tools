use strategies::{
    Strategy,
}; 



pub struct NewStrategyConstructor {
    pub strategy: Strategy,
}

impl NewStrategyConstructor {

    pub fn new() -> Self {
        Self {
            strategy: Strategy::empty()
        }
    }

    pub fn get_form_rows(&self) -> Vec<String> {
        
        let mut rows: Vec<String> = Vec::new();


        rows

    }

}


