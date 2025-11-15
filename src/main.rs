mod core;
mod events;

use core::event_chain::EventChain;
use core::event_context::EventContext;
use core::event_result::EventResult;
use core::fault_tolerance_mode::FaultToleranceMode;
use events::chainable_event::ChainableEvent;
use events::event_middleware::EventMiddleware;

// Example event implementations
struct LogEvent {
    message: String,
}

impl ChainableEvent for LogEvent {
    fn execute(&self, context: &mut EventContext) -> EventResult<()> {
        println!("LOG: {}", self.message);
        context.set("last_log", self.message.clone());
        EventResult::Success(())
    }

    fn name(&self) -> &str {
        "LogEvent"
    }
}

struct CalculateEvent {
    value: i32,
}

impl ChainableEvent for CalculateEvent {
    fn execute(&self, context: &mut EventContext) -> EventResult<()> {
        let current: i32 = context.get("total").unwrap_or(0);
        let new_total = current + self.value;
        context.set("total", new_total);
        println!("CALC: {} + {} = {}", current, self.value, new_total);
        EventResult::Success(())
    }

    fn name(&self) -> &str {
        "CalculateEvent"
    }
}

// Example middleware
struct TimingMiddleware;

impl EventMiddleware for TimingMiddleware {
    fn execute(
        &self,
        event: &dyn ChainableEvent,
        context: &mut EventContext,
        next: &mut dyn FnMut(&mut EventContext) -> EventResult<()>,
    ) -> EventResult<()> {
        let start = std::time::Instant::now();
        let result = next(context);
        let elapsed = start.elapsed();
        println!("  [TIMING] {} took {:?}", event.name(), elapsed);
        result
    }
}

fn main() {
    println!("=== Event Chain Example ===\n");

    // OLD API (still works):
    println!("--- Old API (with boxing) ---");
    let mut old_chain = EventChain::new();
    old_chain.add_event(Box::new(LogEvent {
        message: "Starting old chain".to_string(),
    }));
    old_chain.add_event(Box::new(CalculateEvent { value: 10 }));

    let mut ctx1 = EventContext::new();
    let result1 = old_chain.execute(&mut ctx1);
    println!("Result: {:?}\n", result1.status);

    // NEW API (much cleaner!):
    println!("--- New API (automatic boxing) ---");
    let new_chain = EventChain::new()
        .event(LogEvent {
            message: "Starting new chain".to_string(),
        })
        .event(CalculateEvent { value: 20 })
        .event(CalculateEvent { value: 15 })
        .middleware(TimingMiddleware)
        .with_fault_tolerance(FaultToleranceMode::Lenient);

    let mut ctx2 = EventContext::new();
    let result2 = new_chain.execute(&mut ctx2);
    println!("\nResult: {:?}", result2.status);
    println!("Final total: {}", ctx2.get::<i32>("total").unwrap_or(0));
}
