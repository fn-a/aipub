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

```bash
cargo add aipub
```

### Basic Usage

```rust
use aipub::{
   validate_aic_format, parse_aic, AicInfo,
   AgentCapabilitySpec, TaskCommand, TaskState,
   DiscoveryRequest, DiscoveryResponse,
};

// Validate AIC
let aic = "1.2.156.3088.1.0001.00001.ABC123.000000.0M9Y";
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
# Run the end-to-end basic flow demo
cargo run --example e2eflow

# Run the protocol data model example
cargo run --example basic

# Run tests
cargo test
```

## Basic End-to-End Flow

`examples/e2eflow.rs` wires the whole chain in a single process, and
`tests/e2eflow.rs` asserts every step of it:

```text
ACS registration -> AIC allocation (ontology/entity) -> change-feed sync (DSP)
   -> discovery (natural language / structured filter) -> cross-domain forwarding
   -> EAB issuance and single-use consumption -> ACME account binding
   -> order / certificate issuance -> group ACL & MQ authorization
   -> AIP task dispatch and result return -> unregister / deactivate / revoke
```

## Project Structure

```
aipub/
├── Cargo.toml                 # Workspace + Package config
├── src/
│   └── lib.rs                 # Unified export entry ✅
├── examples/
│   ├── basic.rs               # Protocol data model example ✅
│   └── e2eflow.rs             # Basic end-to-end flow demo ✅
├── tests/                     # Test cases
│   ├── integration.rs         # Test entry: declares module tests + cross-module flows
│   ├── e2eflow.rs             # Basic end-to-end flow test
│   ├── protocols/             # Protocol tests (aic / acs / aip / adp / atr / dsp)
│   ├── registry/              # Registry service tests
│   ├── discovery/             # Discovery service tests
│   ├── certifier/             # Certifier service tests
│   └── authorize/             # Authorize service tests
└── crates/
    ├── protocols/             # Core protocol library ✅
    │   ├── src/
    │   │   ├── aic/           # Agent Identity Code (CRC-16/CCITT-FALSE + generator)
    │   │   ├── acs/           # Agent Capability Specification
    │   │   ├── aip/           # Agent Interaction Protocol (task state machine + JSON-RPC)
    │   │   ├── adp/           # Agent Discovery Protocol (filter evaluation + forwarding guards)
    │   │   ├── atr/           # Trust registry support (EAB HMAC, certificate constants)
    │   │   └── dsp/           # Data sync change feed
    │   └── README.md
    │
    ├── discovery/             # Discovery service (memory storage + keyword matcher + forwarder) ✅
    ├── registry/              # Registry service (AIC allocation + ACS validation + EAB + change feed) ✅
    ├── certifier/             # Certifier service (ACME state machine + CRL/OCSP; real X.509 pending) ✅
    └── authorize/             # Authorize service (MQ auth + group ACL) ✅
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

## Completed functions

### Basic flow wired end to end

- ✅ **Protocol layer**: AIC generation/validation (CRC-16/CCITT-FALSE), ACS validation
  rules, 35 ADP filter operators, AIP task state machine + JSON-RPC models,
  ATR (EAB HMAC) and DSP (change feed)
- ✅ **Registry**: AIC allocation (ontology/entity/derived entity), ACS validation and
  identity metadata rewrite, EAB issuance and single-use consumption, change feed
- ✅ **Discovery**: in-memory storage, keyword matching and ranking, four query modes,
  structured filters, cross-domain forwarding (loop/depth/fanout guards + aggregation),
  idempotent change-feed application
- ✅ **Authorize**: RabbitMQ auth-backend decision rules, group ACL,
  invite/join/remove/disband
- ✅ **Certifier**: ACME flow state machine (account/order/finalize), EAB verification,
  revocation, CRL, OCSP
- ✅ **End to end**: `examples/e2eflow.rs` and `tests/e2eflow.rs`

## Future Plans

### Short-term Goals

1. Productionize Discovery
   - Implement concrete storage backend (PostgreSQL + Redis)
   - Implement HTTP API interface (`POST /discover`)

2. Productionize Registry
   - Implement concrete storage backend
   - Full JSON Schema validation for ACS plus account/approval flow
   - Implement HTTP API interface

### Mid-term Goals

3. Real cryptography in Certifier
   - Integrate rustls / rcgen (or OpenSSL) for CSR parsing and X.509 issuance
   - Emit real DER CRL and OCSP responses
   - Implement ACME JWS verification and nonce management

4. Vector similarity matching
   - Integrate vector database (e.g., Qdrant/Milvus)
   - Implement semantic similarity matching

5. Runnable service processes
   - Provide independently runnable HTTP services and config loading for all four services

### Long-term Goals

6. gRPC support
7. Message queue integration (RabbitMQ, Redis Streams, Kafka)
8. Monitoring and observability (Prometheus, OpenTelemetry)
9. Deployment and operations support (Docker, Kubernetes)

## References

- [GB/Z 185 "Artificial Intelligence Agent Interconnection" National Standard](https://std.samr.gov.cn/gb/search/gbDetailed?id=52A05F8113453761E06397BE0A0AFEB5)
- [ACPs-community Python Implementation](https://atomgit.com/AIP-PUB/ACPs-community)
- [ACPs Protocol Specification Documentation](https://atomgit.com/AIP-PUB/ACPs-community/tree/main/acps-specs)

## License

[![MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

