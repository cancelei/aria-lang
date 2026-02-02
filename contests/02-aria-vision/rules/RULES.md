# Contest #2: Aria Vision Challenge - Detailed Rules

## Submission Guidelines

### Track-Specific Requirements

#### Track A: Language Features

**Scope:**
- Type system components
- Ownership analysis
- Contract verification
- Effect system
- Pattern matching
- Memory management

**Requirements:**
- Implement in Rust (Aria's implementation language)
- Include formal specification
- Provide test suite
- Benchmark performance
- Document algorithm complexity

**Deliverables:**
- Working implementation
- Technical writeup
- Test suite
- Performance analysis
- Integration guide

#### Track B: Tooling

**Scope:**
- LSP server components
- IDE plugins
- Build system
- Debuggers
- Profilers
- Package manager features

**Requirements:**
- Production-ready quality
- Cross-platform support (when applicable)
- User documentation
- Installation guide
- Usage examples

**Deliverables:**
- Installable tool/plugin
- User guide
- Developer documentation
- Demo video
- Integration tests

#### Track C: Standard Library

**Scope:**
- Core data structures
- I/O abstractions
- Networking
- Serialization
- Async runtime
- String/Collection operations

**Requirements:**
- Memory-safe implementation
- Comprehensive tests
- API documentation
- Performance benchmarks
- Cross-platform compatibility

**Deliverables:**
- Library code
- API documentation
- Usage examples
- Test suite
- Performance report

#### Track D: Research & Documentation

**Scope:**
- Type system research
- Memory model analysis
- Compiler optimizations
- Language design studies
- Tutorial content
- Comparative analysis

**Requirements:**
- Academic rigor or practical depth
- Citations and references
- Clear methodology
- Reproducible results (if applicable)
- Practical recommendations

**Deliverables:**
- Research paper OR comprehensive tutorial
- Supporting code/experiments
- References
- Practical applications
- Future work suggestions

## Evaluation Criteria Breakdown

### Alignment with Aria Goals (30 points)
- Addresses core Aria principles (10 pts)
- Fits architectural vision (8 pts)
- Solves real problems (7 pts)
- Future-proof design (5 pts)

### Quality (25 points)
- Code quality/writing quality (10 pts)
- Testing/validation (7 pts)
- Error handling (5 pts)
- Edge cases covered (3 pts)

### Impact (25 points)
- Ecosystem benefit (10 pts)
- Innovation level (8 pts)
- Reusability (7 pts)

### Documentation (20 points)
- Clarity (8 pts)
- Completeness (7 pts)
- Examples (5 pts)

## What We're Looking For

### Track A: Language Features
- Correct implementation of algorithms
- Performance considerations
- Memory safety guarantees
- Integration with existing compiler
- Novel approaches welcome

### Track B: Tooling
- Excellent user experience
- Reliable and fast
- Well-designed APIs
- Extensible architecture
- Professional polish

### Track C: Standard Library
- Idiomatic API design
- Zero-cost abstractions
- Comprehensive coverage
- Excellent documentation
- Real-world usability

### Track D: Research & Documentation
- Clear insights
- Practical applicability
- Rigorous methodology
- Actionable recommendations
- Community value

## Submission Format

### Code Submissions (Tracks A, B, C)
```
your-project/
├── README.md           # Overview
├── DESIGN.md          # Design decisions
├── LICENSE            # MIT/Apache 2.0
├── Cargo.toml         # If Rust
├── src/               # Source code
├── tests/             # Comprehensive tests
├── benches/           # Benchmarks
├── docs/              # Documentation
├── examples/          # Usage examples
└── INTEGRATION.md     # How to integrate
```

### Research Submissions (Track D)
```
your-research/
├── README.md          # Abstract & summary
├── paper.pdf          # Main document
├── code/              # Supporting code
├── data/              # Experimental data
├── figures/           # Diagrams/charts
├── references.bib     # Citations
└── APPLICATIONS.md    # Practical uses
```

## Judging Process

### Phase 1: Technical Review
- Functionality check
- Code review
- Test execution
- Documentation review

### Phase 2: Expert Evaluation
- Three judges per submission
- Independent scoring
- Detailed feedback
- Consensus discussion

### Phase 3: Community Voting (20% weight)
- Public showcase
- Community voting period
- Combined with judge scores

### Phase 4: Final Selection
- Top 3 per track
- Best Overall selection
- Tie-breaking procedures
- Winner announcement

## Integration Path

Winners' submissions may be:
- Integrated into core Aria project
- Published as official packages
- Featured in documentation
- Maintained by core team
- Cited in research

Contributors will:
- Receive attribution
- Be invited to maintainer team
- Get priority on future RFCs
- Have speaking opportunities

## Resources Provided

### Access to Core Team
- Office hours (weekly)
- Code review support
- Architecture guidance
- Technical questions

### Documentation
- [Aria PRD](../PRD-v2.md)
- [Architecture Overview](../docs/designs/)
- [Contributing Guide](../community/CONTRIBUTING.md)
- [Style Guide](../community/STYLE_GUIDE.md)

### Development Tools
- Access to Aria Discord
- CI/CD infrastructure
- Testing resources
- Benchmark infrastructure

## Frequently Asked Questions

**Q: Can I contribute to existing Aria code?**
A: Yes! Improving existing code is valid for Tracks A, B, or C.

**Q: What if my feature is already planned?**
A: Implementations of planned features are welcome! Just do it well.

**Q: Can I combine multiple tracks?**
A: Choose one primary track, but cross-track work is encouraged.

**Q: Is research-only submission valid?**
A: Yes for Track D! Pure research is valuable.

**Q: What about incomplete features?**
A: Better to submit quality partial work than rushed full features.

**Q: Can I get help from core team?**
A: Yes! Office hours and Discord support available.

## Important Dates & Daily Schedule

### Week Timeline
- **Monday 9:00 AM UTC**: Contest kickoff + Track selection
- **Tuesday-Saturday**: Daily implementation sprints
- **Sunday 11:59 PM UTC**: Final submissions due
- **Monday**: Judging + Community voting opens
- **Tuesday**: Winners announced

### Daily Schedule (Every Day)
- **9:00 AM UTC**: Organizer daily update
  - Track-specific challenges
  - Progress highlights from all tracks
  - Technical resources

- **Throughout Day**: Build and collaborate
  - Share commits and progress
  - Code reviews from community
  - Pair programming sessions
  - Real-time help from organizers

- **6:00 PM UTC**: Daily demo session
  - Show what you implemented
  - Get expert feedback
  - See other tracks' progress

See [DAILY_UPDATES.md](../DAILY_UPDATES.md) for complete guide

## Submission Process

1. **Register**: Fill out track registration form
2. **Discuss**: Share your proposal in Discord
3. **Build**: Develop your submission
4. **Get Feedback**: Use office hours
5. **Submit**: Via GitHub PR
6. **Present**: Optional lightning talk
7. **Celebrate**: Winner announcement event

## Recognition Distribution

Winners receive:
- **Visibility**: Featured across aria-lang.dev, blog, social media
- **Authority**: Contributor/maintainer status in the project
- **Influence**: Input on project direction and RFCs
- **Opportunities**: Speaking slots, article features, interviews
- **Legacy**: Your work becomes part of Aria's foundation
- **Community**: Join the core team and shape the future

Team recognition is shared among all members with individual spotlights.

## Questions?

- Email: aria-vision@aria-lang.dev
- Discord: #contest-aria-vision
- GitHub: Issues in contest-submissions repo
- Office Hours: Thursdays 18:00 UTC

---

*"Build what Aria dreams of becoming. Shape the future of safe, expressive programming."*
