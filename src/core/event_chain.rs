use std::fmt;
use crate::core::chain_result::{ChainResult, ChainStatus};
use crate::core::EventContext::EventContext;
use crate::core::EventFailure::EventFailure;
use crate::core::EventResult::EventResult;
use crate::core::FaultToleranceMode::FaultToleranceMode;
use crate::events::ChainableEvent::ChainableEvent;
use crate::events::EventMiddleware::EventMiddleware;

/// Main EventChain orchestrator
pub struct EventChain {
    events: Vec<Box<dyn ChainableEvent>>,
    middlewares: Vec<Box<dyn EventMiddleware>>,
    fault_tolerance: FaultToleranceMode,
}

impl EventChain {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            middlewares: Vec::new(),
            fault_tolerance: FaultToleranceMode::Strict,
        }
    }

    pub fn with_fault_tolerance(mut self, mode: FaultToleranceMode) -> Self {
        self.fault_tolerance = mode;
        self
    }

    /// Add an event to the chain (fluent API - consumes self)
    pub fn event<E: ChainableEvent + 'static>(mut self, event: E) -> Self {
        self.events.push(Box::new(event));
        self
    }

    /// Add a middleware to the chain (fluent API - consumes self)
    pub fn middleware<M: EventMiddleware + 'static>(mut self, middleware: M) -> Self {
        self.middlewares.push(Box::new(middleware));
        self
    }

    /// Legacy method for adding boxed events (mutable reference API)
    pub fn add_event(&mut self, event: Box<dyn ChainableEvent>) -> &mut Self {
        self.events.push(event);
        self
    }

    /// Legacy method for adding boxed middleware (mutable reference API)
    pub fn use_middleware(&mut self, middleware: Box<dyn EventMiddleware>) -> &mut Self {
        self.middlewares.push(middleware);
        self
    }

    pub fn execute(&self, context: &mut EventContext) -> ChainResult {
        let mut failures = Vec::new();

        for event in &self.events {
            // Build middleware pipeline (LIFO - last registered executes first)
            let result = self.execute_with_middleware(event.as_ref(), context);

            if result.is_failure() {
                let failure = EventFailure::new(
                    event.name().to_string(),
                    result.get_error().unwrap_or("Unknown error").to_string(),
                );
                failures.push(failure);

                match self.fault_tolerance {
                    FaultToleranceMode::Strict => {
                        return ChainResult::failure(failures);
                    }
                    FaultToleranceMode::Lenient | FaultToleranceMode::BestEffort => {
                        // Continue execution
                        continue;
                    }
                }
            }
        }

        if failures.is_empty() {
            ChainResult::success()
        } else {
            ChainResult::partial_success(failures)
        }
    }

    fn execute_with_middleware(
        &self,
        event: &dyn ChainableEvent,
        context: &mut EventContext,
    ) -> EventResult<()> {
        if self.middlewares.is_empty() {
            return event.execute(context);
        }

        // Execute middleware in reverse order by recursively building the call stack
        self.execute_middleware_recursive(0, event, context)
    }

    fn execute_middleware_recursive(
        &self,
        middleware_index: usize,
        event: &dyn ChainableEvent,
        context: &mut EventContext,
    ) -> EventResult<()> {
        if middleware_index >= self.middlewares.len() {
            // Base case: execute the actual event
            return event.execute(context);
        }

        // Get the current middleware (reverse order)
        let middleware_idx = self.middlewares.len() - 1 - middleware_index;
        let middleware = &self.middlewares[middleware_idx];

        // Create a closure that calls the next middleware (or event)
        let mut next = |ctx: &mut EventContext| -> EventResult<()> {
            self.execute_middleware_recursive(middleware_index + 1, event, ctx)
        };

        // Execute this middleware with the next closure
        middleware.execute(event, context, &mut next)
    }
}

impl Default for EventChain {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ChainStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChainStatus::Completed => write!(f, "COMPLETED"),
            ChainStatus::CompletedWithWarnings => write!(f, "COMPLETED_WITH_WARNINGS"),
            ChainStatus::Failed => write!(f, "FAILED"),
        }
    }
}
