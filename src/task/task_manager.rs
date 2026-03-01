use alloc::{boxed::Box, collections::vec_deque::VecDeque};

use crate::task::task::Task;

const MAX_THREAD: usize = 20;

pub struct GlobalTaskState {
    pub Task_list: VecDeque<Task>,
    pub Current_task: Option<Box<Task>>,
}

impl GlobalTaskState {
    pub fn new() -> Self {
        GlobalTaskState {
            Task_list: VecDeque::with_capacity(MAX_THREAD),
            Current_task: None,
        }
    }
}
