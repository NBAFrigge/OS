use core::future::Ready;

use alloc::{boxed::Box, collections::vec_deque::VecDeque};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{
    idt::interrupt::TICKS,
    task::task::{self, Task},
};

const MAX_THREAD: usize = 20;

pub struct TaskManager {
    pub task_list: VecDeque<Box<Task>>,
    pub current_task: Option<Box<Task>>,
}

impl TaskManager {
    pub fn new(max_thread: usize) -> Self {
        TaskManager {
            task_list: VecDeque::with_capacity(max_thread),
            current_task: None,
        }
    }

    pub fn rotate(&mut self, old_rsp: usize) -> usize {
        if let Some(task) = self.current_task.as_mut() {
            task.saved_stack_pointer = old_rsp;
        } else {
            return 0;
        }

        if self.task_list.is_empty() {
            return self.current_task.as_ref().unwrap().saved_stack_pointer;
        }

        for _ in 0..self.task_list.len() {
            let mut next_task = self.task_list.pop_front().unwrap();

            if next_task.state == task::State::Waiting
                && next_task.wake_on_tick > 0
                && next_task.wake_on_tick <= TICKS.load(core::sync::atomic::Ordering::Relaxed)
            {
                next_task.wake_on_tick = 0;
                next_task.state = task::State::Ready;
            }

            if next_task.state == task::State::Ready && next_task.id != 0 {
                let mut old_task = self.current_task.take().unwrap();

                if old_task.state == task::State::Running {
                    old_task.state = task::State::Ready;
                }

                next_task.state = task::State::Running;
                let new_ptr = next_task.saved_stack_pointer;

                self.task_list.push_back(old_task);
                self.current_task = Some(next_task);
                return new_ptr;
            } else {
                self.task_list.push_back(next_task);
            }
        }

        let idle_index = self.task_list.iter().position(|t| t.id == 0);
        match idle_index {
            Some(index) => {
                if self.current_task.as_ref().unwrap().id == 0 {
                    return self.current_task.as_ref().unwrap().saved_stack_pointer;
                }

                let mut idle_task = self.task_list.remove(index).unwrap();
                let mut old_task = self.current_task.take().unwrap();

                if old_task.state == task::State::Running {
                    old_task.state = task::State::Ready;
                }

                idle_task.state = task::State::Running;

                let new_ptr = idle_task.saved_stack_pointer;

                self.task_list.push_back(old_task);
                self.current_task = Some(idle_task);
                new_ptr
            }
            None => self.current_task.as_ref().unwrap().saved_stack_pointer,
        }
    }
}

lazy_static! {
    pub static ref GLOBAL_TASK_MANAGER: Mutex<TaskManager> =
        Mutex::new(TaskManager::new(MAX_THREAD));
}
