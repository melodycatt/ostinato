use crate::State;

pub trait Component {
    fn update(state: &mut State);
}