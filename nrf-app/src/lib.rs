#![no_std]

use heapless::mpmc::Q64;
use cortex_m::asm;

pub enum AsyncStep {

  // Low level steps (if needed, e.g. called from ISR): 

  // the simplest step: run a thunk
  StepUnit { run: fn() -> () },

  // A generic step: run a fn with a u32 parameter
  StepU32 { run: fn(u32) -> (), arg: u32 },

  // Or with two u32 parameters
  Step2U32 { run: fn(u32, u32) -> (), arg0: u32, arg1: u32 }, 

  // High level steps for the state machine:

  // Perform a defined command 
  Perform { command: state_machine::Command },

  // Notify the state machine that a defined event occured 
  Notify { event: state_machine::Event },

  // Stop processing steps
  Stop
} 

type AsyncQueue = Q64<AsyncStep>;
static DEFAULT_ASYNC_QUEUE: AsyncQueue = Q64::new();

impl AsyncStep {

  pub fn enqueue(self, queue: &AsyncQueue) -> () {
    queue.enqueue(self).ok();
  }

  pub fn enqueue_default(self) -> () { 
    self.enqueue(&DEFAULT_ASYNC_QUEUE); 
  }

  pub fn run_default(start: state_machine::State, handler: &mut dyn state_machine::CommandHandler) -> state_machine::State { 
    AsyncStep::run_queue(&DEFAULT_ASYNC_QUEUE, start, handler)
  }

  pub fn run_queue(queue: &AsyncQueue, start: state_machine::State, handler: &mut dyn state_machine::CommandHandler) -> state_machine::State {
    use crate::AsyncStep::*;
    use crate::state_machine::*;

    let mut state = start;
    
    loop {
      if let Some(step) = queue.dequeue() {
        match step {
          StepUnit { run } => run(),
          StepU32 { run, arg } => run(arg),
          Step2U32 { run, arg0, arg1 } => run(arg0, arg1),
          Perform { command } =>   handler.handle(&command, queue),
          Notify { event } => {
            let (o, t) = transition(&state, &event);
            match t {
              Transition::Next(s) => state = s,
              Transition::Same => ()
            }
            if let Some(c) = o {
              AsyncStep::Perform { command: c }.enqueue(queue)
            }
          },
          Stop => return state
        }
      } else {
        asm::wfi();
      }
    }
  }
}

impl state_machine::EventNotifier for AsyncQueue {
  fn notify( &self, e: state_machine::Event ) -> () { 
    AsyncStep::Notify { event: e }.enqueue(self)
  }
}


mod state_machine {

  pub enum State { Standby /* and other states */  }
  pub enum Command { /* various commands */ }
  pub enum Event {  /* various events */ }
  
  pub trait CommandHandler {
    fn handle( &mut self, command: &Command, notifier: & dyn EventNotifier) -> ();
  }

  pub trait EventNotifier {
    fn notify( &self, event: Event ) -> ();
  }

  pub enum Transition {
    Next(State),
    Same
  }

  pub fn transition(_s: &State, _e: &Event) -> (Option<Command>, Transition) { (None, Transition::Same) }

}