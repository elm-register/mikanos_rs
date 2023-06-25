use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::vec;
use core::cell::OnceCell;
use core::sync::atomic::{AtomicU8, Ordering};

use crate::context::arch::x86_64::Context;
use crate::error::KernelResult;
use crate::interrupt::interrupt_message::TaskMessage;
use crate::task::list::TaskList;
use crate::task::priority_level::PriorityLevel;
use crate::task::status::Status;
use crate::task::status::Status::{Pending, Running, Sleep};
use crate::{interrupt, kernel_error};

mod list;
pub mod priority_level;
mod status;
mod switch;


pub struct GlobalTaskManger(OnceCell<TaskManager>);


impl GlobalTaskManger {
    #[inline(always)]
    pub const fn uninit() -> Self {
        Self(OnceCell::new())
    }


    pub fn init(&self) -> Result<(), TaskManager> {
        self.0.set(TaskManager::new())
    }


    pub fn new_task(&mut self, priority_level: PriorityLevel, rip: u64, rsi: u64) {
        unsafe {
            self.0
                .get_mut()
                .unwrap()
                .new_task(priority_level)
                .init_context(rip, rsi);
        }
    }


    pub fn switch_task(&mut self) {
        if let Some(manager) = self.0.get_mut() {
            manager.switch_task().unwrap();
        }
    }


    pub fn send_message_at(&mut self, task_id: u64, message: TaskMessage) -> KernelResult {
        self.lock()?
            .send_message_at(task_id, message)
    }


    pub fn receive_message_at(&mut self, task_id: u64) -> Option<TaskMessage> {
        self.lock()
            .ok()?
            .receive_message_at(task_id)
    }


    pub fn sleep_at(&mut self, task_id: u64) -> KernelResult {
        self.lock()?.sleep_at(task_id)
    }


    pub fn wakeup_at(&mut self, task_id: u64) -> KernelResult {
        self.lock()?
            .wakeup_at(task_id)
    }


    fn lock(&mut self) -> KernelResult<&mut TaskManager> {
        self.0
            .get_mut()
            .ok_or(kernel_error!("Not Initialized Task Manager"))
    }
}


unsafe impl Sync for GlobalTaskManger {}


#[derive(Default, Debug)]
pub struct TaskManager {
    tasks: TaskList,
}


impl TaskManager {
    #[inline(always)]
    pub fn new() -> Self {
        let mut tasks = TaskList::new();
        tasks.push(Task::new_main());
        Self { tasks }
    }


    pub fn send_message_at(&mut self, task_id: u64, message: TaskMessage) -> KernelResult {
        let task = self.tasks.find_mut(task_id)?;

        if task.status().is_sleep() {
            task.store_status(Pending);
        }

        task.send_message(message);

        Ok(())
    }


    pub fn receive_message_at(&mut self, task_id: u64) -> Option<TaskMessage> {
        self.tasks
            .find_mut(task_id)
            .unwrap()
            .receive_message()
    }


    pub fn new_task(&mut self, priority_level: PriorityLevel) -> &mut Task {
        let task = self.create_task(priority_level);
        let id = task.id;
        self.tasks.push(task);

        self.tasks
            .find_mut(id)
            .unwrap()
    }


    pub fn wakeup_at(&mut self, task_id: u64) -> KernelResult {
        self.tasks.wakeup_at(task_id)
    }


    pub fn sleep_at(&mut self, task_id: u64) -> KernelResult {
        self.tasks.sleep_at(task_id)
    }


    pub fn switch_task(&mut self) -> KernelResult {
        self.tasks
            .next_switch_command()?
            .switch_if_need(Pending);

        Ok(())
    }


    pub fn switch_ignore_priority(&mut self) -> KernelResult {
        self.tasks
            .next_switch_command()?
            .switch_and_pending();

        Ok(())
    }


    #[inline]
    fn create_task(&self, priority_level: PriorityLevel) -> Task {
        Task::new(self.tasks.len() as u64, priority_level)
    }
}


#[derive(Debug)]
pub struct Task {
    id: u64,
    priority_level: PriorityLevel,
    context: Context,
    stack: Box<[u8]>,
    messages: VecDeque<TaskMessage>,
    status: AtomicU8,
}


impl Task {
    pub fn new_main() -> Self {
        Self {
            id: 0,
            priority_level: PriorityLevel::new(3),
            context: Context::uninit(),
            stack: vec![0; 65_536].into_boxed_slice(),
            messages: VecDeque::new(),
            status: AtomicU8::new(Running as u8),
        }
    }


    pub fn new(id: u64, priority_level: PriorityLevel) -> Self {
        Self {
            id,
            priority_level,
            context: Context::uninit(),
            stack: vec![0; 65_536].into_boxed_slice(),
            messages: VecDeque::new(),
            status: AtomicU8::new(Pending as u8),
        }
    }


    #[inline(always)]
    pub fn store_status(&self, status: Status) {
        interrupt::asm::without_interrupt(|| {
            self.status
                .store(status as u8, Ordering::Relaxed);
        });
    }


    #[inline(always)]
    pub fn status(&self) -> Status {
        match self
            .status
            .load(Ordering::Relaxed)
        {
            0 => Sleep,
            1 => Pending,
            2 => Running,
            _ => panic!("Invalid Status"),
        }
    }

    #[inline(always)]
    pub fn switch_to(&self, next: &Task) {
        self.context
            .switch_to(&next.context);
    }


    pub unsafe fn init_context(&mut self, rip: u64, rsi: u64) {
        let task_end = self.stack.as_ptr_range().end as u64;
        let rsp = (task_end & !0xF) - 8;
        self.context
            .init_context(rip, self.id, rsi, rsp);
    }


    pub fn receive_message(&mut self) -> Option<TaskMessage> {
        self.messages.pop_front()
    }


    pub fn send_message(&mut self, message: TaskMessage) {
        self.messages
            .push_back(message);
    }
}

#[cfg(test)]
mod tests {}
