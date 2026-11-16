// Tests for `Model`: loads the model from `data/223/model.bin`.

use acrostic_generator::model::Model;

#[test]
fn read_model() {
    Model::get();
}

// #[test]
// fn test_sort() {
//     let model = Model::get();
//     println!("{:?}", &model.sorted("berkeley", Part::Noun)[0..100]);
//     println!("{:?}", &model.sorted("berkeley", Part::Propn)[0..100]);
// }
