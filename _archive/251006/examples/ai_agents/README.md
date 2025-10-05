# Manimani - AI Agent Framework

Manimani is a **Jsonnet-only AI agent framework** built on top of the Kotoba Process Network Graph Model. It provides a complete AI agent ecosystem without requiring any Rust code - everything is implemented in Jsonnet DSL.

## Features

- **Pure Jsonnet Implementation**: No Rust code required - everything is Jsonnet
- **Multiple Agent Types**: Chatbots, ReAct agents, Chain-based workflows
- **Tool Integration**: Built-in support for external tools and APIs
- **Memory Management**: Conversation context and state management
- **Process Network Integration**: Built on Kotoba's graph-based architecture

## Architecture

Manimani uses the following Jsonnet stdlib extensions:

### AI Functions
- `std.ai.httpGet(url, headers={})` - HTTP GET requests
- `std.ai.httpPost(url, body, headers={})` - HTTP POST requests
- `std.ai.callModel(model, messages, options={})` - Call AI models

### Tool Functions
- `std.tool.execute(command, args=[], env={})` - Execute external commands

### Memory Functions
- `std.memory.get(key)` - Retrieve from memory
- `std.memory.set(key, value)` - Store in memory

### Agent Functions
- `std.agent.create(type, config)` - Create an agent
- `std.agent.execute(agent, input)` - Execute agent

### Chain Functions
- `std.chain.create(steps)` - Create processing chain
- `std.chain.execute(chain, input)` - Execute chain

## Examples

### Simple Chatbot

```jsonnet
// chatbot.kotobas
{
  name: "SimpleChatbot",
  config: {
    model: "gpt-3.5-turbo",
    temperature: 0.7,
    system_prompt: "You are a helpful AI assistant.",
  },

  run(input):: {
    local agent = std.agent.create("chatbot", self.config),
    local response = std.agent.execute(agent, input),
    response
  },
}
```

### ReAct Agent

```jsonnet
// react_agent.kotobas
{
  name: "ReActAgent",
  config: {
    model: "gpt-4",
    tools: {
      get_weather: { /* tool definition */ },
      calculate: { /* tool definition */ },
    },
  },

  run(input):: {
    // ReAct implementation with reasoning + acting
    local agent = std.agent.create("react", self.config),
    local result = std.agent.execute(agent, input),
    result
  },
}
```

### Processing Chain

```jsonnet
// chain_example.kotobas
{
  name: "ResearchChain",
  steps: [
    {
      name: "research",
      type: "llm_call",
      config: { model: "gpt-4", prompt_template: "Research {topic}" },
    },
    {
      name: "summarize",
      type: "llm_call",
      config: { model: "gpt-3.5-turbo", prompt_template: "Summarize {research_output}" },
    },
  ],

  run(topic):: {
    local chain = std.chain.create(self.steps),
    local result = std.chain.execute(chain, { topic: topic }),
    result
  },
}
```

## Running Agents

```bash
# Evaluate a .kotobas agent file
kotoba evaluate chatbot.kotobas

# Execute agent with input
kotoba run chatbot.kotobas --input "Hello, how are you?"

# For chains
kotoba run chain_example.kotobas --input "artificial intelligence"
```

## File Format (.kotobas)

.kotobas files are Jsonnet files with the following structure:

```jsonnet
{
  // Metadata
  name: "AgentName",
  version: "1.0.0",
  description: "Agent description",

  // Configuration
  config: {
    // Agent-specific configuration
  },

  // Optional: Tools definition
  tools: [
    {
      name: "tool_name",
      description: "Tool description",
      parameters: { /* JSON Schema */ },
    },
  ],

  // Agent logic (functions)
  init()::
    // Initialization logic
    std.agent.create("agent_type", self.config),

  run(input)::
    // Main execution logic
    local agent = self.init(),
    local result = std.agent.execute(agent, input),
    result,
}
```

## Integration with Process Network

Manimani agents are fully integrated with Kotoba's Process Network Graph Model:

- Each agent component has a unique CID hash
- Agent execution follows topological order
- Error recovery uses reverse topological sort
- All agent state is persisted in the graph

## Current Status

This is a **Jsonnet-only prototype** that demonstrates the concept. The actual runtime execution requires:

1. **Jsonnet evaluator extensions** for real HTTP/AI API calls
2. **Async execution support** in Jsonnet
3. **External tool execution** capabilities
4. **Persistent memory storage**

The framework provides the complete DSL and architecture - the runtime implementation would handle the actual execution of these Jsonnet-defined agents.
