# AIP Agent Interconnection Protocol

[中文](README.md) | **English**

Rust implementation based on GB/Z 185 "Artificial Intelligence Agent Interconnection" national standard.

## Introduction

This project is a Rust implementation based on the ACPs (Agent Collaboration Protocols) specification, developed by referencing the Python implementation. ACPs is a standardized interaction protocol system designed and implemented to ensure efficient collaboration among heterogeneous agents and support diverse agent interconnection applications.

### Protocol System

| Protocol | Description | Module Path |
|----------|-------------|-------------|
| **AIC** | Agent Identity Code | `aipub::protocols::aic` |
| **ACS** | Agent Capability Specification | `aipub::protocols::acs` |
| **AIP** | Agent Interaction Protocol | `aipub::protocols::aip` |
| **ADP** | Agent Discovery Protocol | `aipub::protocols::adp` |

### Service Components

| Service | Description | Module Path |
|---------|-------------|-------------|
| **Discovery** | Agent Discovery Service | `aipub::discovery` |
| **Registry** | Agent Registry Service | `aipub::registry` |
| **Certifier** | Certificate Service (CA) | `aipub::certifier` |
| **Authorize** | Authorization Service | `aipub::authorize` |

## Quick Start

### Add Dependency

```toml
[dependencies]
aipub = "0.1.0"
```

### Basic Usage

```rust
use aipub::{
   validate_aic_format, parse_aic, AicInfo,
   AgentCapabilitySpec, TaskCommand, TaskState,
   DiscoveryRequest, DiscoveryResponse,
};

// Validate AIC
let aic = "1.2.156.3088.1.K9.ABCDEF.123456789.000000000.ABCD";
if let Ok(info) = validate_aic_format(aic) {
   println!("AIC valid: {}", info);
   println!("Is ontology: {}", info.is_ontology());
}

// Create discovery request
let request = DiscoveryRequest::new()
   .with_query("I need an agent for Beijing food recommendations")
   .with_limit(10);
```

### Run Examples

```bash
# Run basic example
cargo run --example basic

# Run tests
cargo test
```

Example output:
```
AIP Agent Interconnection Protocol v0.1.0
Protocol version: 02.01
AIC OID prefix: 1.2.156.3088

=== AIC Example ===
✓ AIC format valid
  - version: 1
  - ARSP: K9
  - is_ontology: true

=== ACS Example ===
✓ ACS validation passed
✓ ACS serialization successful (689 bytes)

=== AIP Example ===
✓ Task command created
✓ Task result created

=== ADP Example ===
✓ Discovery request created
✓ Discovery response created
```

## Project Structure

```
aipub/
├── Cargo.toml                 # Workspace + Package config
├── src/
│   └── lib.rs                 # Unified export entry ✅
├── examples/
│   └── basic.rs               # Basic usage example ✅
├── tests/                     # Test cases ✅
└── crates/
    ├── protocols/             # Core protocol library ✅
    │   ├── src/
    │   │   ├── aic/           # Agent Identity Code
    │   │   ├── acs/           # Agent Capability Specification
    │   │   ├── aip/           # Agent Interaction Protocol
    │   │   └── adp/           # Agent Discovery Protocol
    │   └── README.md
    │
    ├── discovery/             # Discovery service framework 🚧
    │   └── README.md
    ├── registry/              # Registry service framework 🚧
    │   └── README.md
    ├── certifier/             # Certifier service framework 🚧
    │   └── README.md
    └── authorize/             # Authorize service framework 🚧
        └── README.md
```

## Protocol Specifications

### AIC - Agent Identity Code

AIC (Agent Identity Code) is a globally unique identifier for agents, using a 10-level OID structure:

```
1.2.156.3088.{version}.{arsp}.{vendor}.{ontology_sn}.{instance_sn}.{crc}
```

- **Levels 1-4**: OID prefix `1.2.156.3088` (assigned by national OID registration center)
- **Level 5**: Version number (Base36, 1 digit)
- **Level 6**: ARSP registration service provider number (Base36, 1-6 digits)
- **Level 7**: Vendor number (Base36, 1-6 digits)
- **Level 8**: Ontology SN serial number (Base36, 1-9 digits)
- **Level 9**: Instance SN serial number (Base36, 1-9 digits, all 0 indicates ontology AIC)
- **Level 10**: CRC checksum (Base36, fixed 4 digits)

### ACS - Agent Capability Specification

ACS (Agent Capability Specification) completely describes the agent's identity information, functional capabilities, technical characteristics, and service interfaces.

Main components:
- Identity information (aic, active, lastModifiedTime)
- Basic description (name, description, version)
- Provider information (provider)
- Security schemes (securitySchemes)
- Service endpoints (endPoints)
- Capabilities (capabilities)
- Skills list (skills)

### AIP - Agent Interaction Protocol

AIP (Agent Interaction Protocol) defines interaction patterns between agents:

- **Task Command** (TaskCommand): Commands sent from Leader to Partner
- **Task Result** (TaskResult): Results returned from Partner to Leader
- **Task State** (TaskState): accepted, working, completed, failed, etc.
- **Data Item** (DataItem): text, file, structured data

### ADP - Agent Discovery Protocol

ADP (Agent Discovery Protocol) defines the agent discovery process:

- **Discovery Request** (DiscoveryRequest): Contains query, filter conditions, forwarding config
- **Discovery Response** (DiscoveryResponse): Returns matched agent list and ACS data
- **Filter Conditions** (DiscoveryFilter): Supports rich operators and logical combinations
- **Forwarding Mechanism**: Supports cross-domain discovery and chain forwarding

## Technical Features

### Rust Implementation Advantages

- ✅ **Type Safety**: Ensures protocol data correctness using Rust type system
- ✅ **Zero-cost Abstraction**: High-performance protocol processing
- ✅ **Memory Safety**: Compile-time memory safety guarantee without GC
- ✅ **Concurrency Safety**: Efficient concurrency using async/await and tokio
- ✅ **Error Handling**: Clear error messages using Result and thiserror

### Protocol Implementation Highlights

- ✅ **Complete Serialization Support**: JSON serialization/deserialization using serde
- ✅ **Builder Pattern**: Fluent API building experience
- ✅ **Validation Logic**: Built-in data validation ensuring protocol compliance
- ✅ **Detailed Documentation**: Each module has detailed documentation and examples
- ✅ **Test Coverage**: Complete test coverage for core functionality

## Future Plans

### Short-term Goals

1. Complete Discovery service implementation
   - Implement concrete storage backend (PostgreSQL + Redis)
   - Implement keyword matcher
   - Implement HTTP API interface

2. Complete Registry service implementation
   - Implement concrete storage backend
   - Implement EAB credential verification
   - Implement HTTP API interface

### Mid-term Goals

3. Complete Certifier service implementation
   - Integrate OpenSSL or rustls
   - Implement ACME protocol
   - Implement certificate issuance and revocation

4. Complete Authorize service implementation
   - Implement ACL rule engine
   - Implement group management
   - Implement message queue authentication backend

5. Vector similarity matching
   - Integrate vector database (e.g., Qdrant/Milvus)
   - Implement semantic similarity matching

### Long-term Goals

6. gRPC support
7. Message queue integration (RabbitMQ, Redis Streams, Kafka)
8. Monitoring and observability (Prometheus, OpenTelemetry)
9. Deployment and operations support (Docker, Kubernetes)

## Parts Not Directly Convertible

The following parts use pseudocode and detailed comments due to Rust limitations:

1. **Dynamic Type Handling**: Use `serde_json::Value` + comment explanation
2. **Async Generators**: Use `Stream` trait + comment explanation
3. **Metaprogramming**: Use procedural macros + comment explanation

## References

- [GB/Z 185 "Artificial Intelligence Agent Interconnection" National Standard](https://std.samr.gov.cn/gb/search/gbDetailed?id=52A05F8113453761E06397BE0A0AFEB5)
- [ACPs-community Python Implementation](https://atomgit.com/AIP-PUB/ACPs-community)
- [ACPs Protocol Specification Documentation](https://atomgit.com/AIP-PUB/ACPs-community/tree/main/acps-specs)

## License

[![MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

