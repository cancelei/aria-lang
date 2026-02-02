# Contributing to Aria Lang

Thank you for your interest in contributing to Aria! This guide will help you get started.

## Code of Conduct

By participating in this project, you agree to abide by our [Code of Conduct](./CODE_OF_CONDUCT.md). Please read it before contributing.

## Ways to Contribute

### 1. Code Contributions
- Compiler improvements
- Standard library features
- Tooling and IDE support
- Bug fixes
- Performance optimizations

### 2. Documentation
- Tutorial content
- API documentation
- Architecture guides
- Translation efforts
- Example programs

### 3. Community
- Answer questions on Discord
- Write blog posts
- Give talks
- Create video tutorials
- Help with onboarding

### 4. Testing & Quality
- Write tests
- Report bugs
- Improve error messages
- Performance benchmarking
- Fuzzing

### 5. Design & Research
- Propose language features (RFCs)
- Research type systems
- Study memory models
- Compare with other languages
- Prototype ideas

## Getting Started

### 1. Set Up Development Environment

```bash
# Clone the repository
git clone https://github.com/cancelei/aria-lang.git
cd aria-lang

# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build the project
cargo build

# Run tests
cargo test

# Run examples
cargo run --example hello_world
```

### 2. Find Something to Work On

- Check [Good First Issues](https://github.com/cancelei/aria-lang/labels/good%20first%20issue)
- Browse [Help Wanted](https://github.com/cancelei/aria-lang/labels/help%20wanted)
- Look at the [Roadmap](../PRD-v2.md)
- Join Discord and ask what's needed

### 3. Make Your Contribution

1. **Create an Issue** (if one doesn't exist)
   - Describe the problem or feature
   - Discuss approach with maintainers
   - Get approval before large changes

2. **Fork and Branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

3. **Write Code**
   - Follow our [Style Guide](./STYLE_GUIDE.md)
   - Write tests
   - Update documentation
   - Keep commits focused

4. **Test Locally**
   ```bash
   cargo test
   cargo clippy
   cargo fmt -- --check
   ```

5. **Submit Pull Request**
   - Clear title and description
   - Reference related issues
   - Explain your changes
   - Add tests and docs

## Pull Request Guidelines

### PR Title Format
```
<type>(<scope>): <description>

Types:
- feat: New feature
- fix: Bug fix
- docs: Documentation
- style: Formatting
- refactor: Code restructuring
- perf: Performance improvement
- test: Add tests
- chore: Build/tooling changes
```

### PR Description Template
```markdown
## Description
Brief explanation of changes

## Motivation
Why is this change needed?

## Changes
- Item 1
- Item 2

## Testing
How was this tested?

## Checklist
- [ ] Tests added/updated
- [ ] Documentation updated
- [ ] Changelog updated
- [ ] Follows style guide
```

### Review Process

1. **Automated Checks**
   - CI tests must pass
   - Clippy warnings addressed
   - Format check passes

2. **Code Review**
   - At least one maintainer approval
   - Address feedback
   - Update as requested

3. **Merge**
   - Squash and merge typically
   - Linear history preferred
   - Credit preserved

## Development Guidelines

### Code Style

- **Rust**: Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- **Formatting**: Use `cargo fmt`
- **Linting**: Pass `cargo clippy`
- **Comments**: Explain "why", not "what"
- **Tests**: Required for new features

### Commit Messages

```
<type>: <subject>

<body>

<footer>
```

Example:
```
feat: implement basic type inference

Add Hindley-Milner type inference for simple expressions.
This enables automatic type deduction for variables and
function return types.

Closes #123
```

### Testing

- **Unit tests**: Test individual functions
- **Integration tests**: Test component interactions
- **Example tests**: Ensure examples work
- **Benchmarks**: For performance-critical code

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_inference() {
        let expr = parse("let x = 42");
        let ty = infer_type(&expr);
        assert_eq!(ty, Type::Int);
    }
}
```

### Documentation

- **Public APIs**: Must have doc comments
- **Examples**: Include usage examples
- **Architecture**: Document design decisions
- **Changelog**: Update CHANGELOG.md

```rust
/// Infers the type of an expression.
///
/// This function uses Hindley-Milner type inference to determine
/// the most general type for the given expression.
///
/// # Examples
///
/// ```
/// let expr = parse("42");
/// let ty = infer_type(&expr);
/// assert_eq!(ty, Type::Int);
/// ```
///
/// # Errors
///
/// Returns `TypeError` if the expression has type conflicts.
pub fn infer_type(expr: &Expr) -> Result<Type, TypeError> {
    // Implementation
}
```

## RFC Process

For significant changes, follow the RFC (Request for Comments) process:

1. **Write RFC**
   - Copy `rfcs/0000-template.md`
   - Fill in details
   - Explain motivation and design

2. **Submit PR**
   - Add to `rfcs/` directory
   - Title: `RFC: Your Feature Name`

3. **Discussion**
   - Community feedback period (2 weeks minimum)
   - Revise based on feedback
   - Reach consensus

4. **Approval**
   - Core team review
   - Accept, reject, or postpone
   - Merge if accepted

5. **Implementation**
   - Reference RFC in implementation PRs
   - Follow approved design
   - Update RFC if needed

## Community Channels

- **Discord**: [Join here](link) - Real-time chat
- **GitHub Discussions**: Design conversations
- **Twitter**: [@aria_lang](link) - Announcements
- **Reddit**: [r/arialang](link) - Community posts
- **Blog**: [blog.aria-lang.dev](link) - Deep dives

## Recognition

Contributors are recognized in:
- CONTRIBUTORS.md file
- Release notes
- Annual contributor spotlight
- Speaking opportunities
- Maintainer invitations (for consistent contributors)

## Getting Help

- **Discord**: Ask in #contributing channel
- **Office Hours**: Weekly maintainer Q&A
- **Mentorship**: Request a mentor for large features
- **Documentation**: Read the [Architecture Guide](../docs/ARCHITECTURE.md)

## License

By contributing, you agree that your contributions will be licensed under:
- **Code**: MIT OR Apache-2.0
- **Documentation**: CC-BY-4.0

---

## Thank You!

Every contribution, no matter how small, helps make Aria better. Whether you're fixing a typo, writing a tutorial, or implementing a new feature, you're part of building the future of programming.

*"Together, we build the language we want to use."*
