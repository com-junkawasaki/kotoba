---
name: Bug Report
about: Report a bug or unexpected behavior
title: "[BUG] Brief description of the issue"
labels: ["bug", "triage"]
assignees: ""
---

## Bug Report

### Description
A clear and concise description of the bug.

### Steps to Reproduce
Steps to reproduce the behavior:
1. Go to '...'
2. Click on '....'
3. Scroll down to '....'
4. See error

### Expected Behavior
A clear and concise description of what you expected to happen.

### Actual Behavior
What actually happened instead.

### Environment
- **KotobaDB Version**: [e.g., v1.2.3]
- **Rust Version**: [e.g., 1.70.0]
- **OS**: [e.g., Ubuntu 22.04, macOS 13.0, Windows 11]
- **Architecture**: [e.g., x86_64, aarch64]
- **Installation Method**: [e.g., cargo install, from source, Docker]

### Configuration
If applicable, provide your KotobaDB configuration:
```toml
[database]
engine = "lsm"
# ... other configuration
```

### Logs
If applicable, provide relevant log output:
```
[2024-01-01T12:00:00Z INFO  kotoba::db] Starting KotobaDB v1.2.3
[2024-01-01T12:00:01Z ERROR kotoba::query] Query execution failed: ...
```

### Additional Context
- **Database Size**: [e.g., number of nodes/edges, storage size]
- **Workload**: [e.g., read-heavy, write-heavy, mixed]
- **Concurrent Users**: [e.g., number of concurrent connections]
- **Reproduction Rate**: [e.g., always, sometimes, rare]
- **Impact**: [e.g., crashes, data corruption, performance degradation]

### Possible Solution
If you have any ideas about how to fix this bug, please describe them here.

### Checklist
- [ ] I have searched existing issues for similar bugs
- [ ] I have provided a minimal reproduction case
- [ ] I have included relevant configuration and logs
- [ ] I have indicated the severity and impact of the bug
