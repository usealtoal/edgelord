//! LLM adapter modules.
//!
//! Provides implementations of the [`Llm`](crate::port::outbound::llm::Llm) trait
//! for various large language model providers including Anthropic Claude and OpenAI.

pub mod anthropic;
pub mod client;
pub mod openai;
