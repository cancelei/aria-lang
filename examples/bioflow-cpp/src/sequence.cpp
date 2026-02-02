#include "bioflow/sequence.hpp"
#include <cctype>
#include <sstream>
#include <iostream>

namespace bioflow {

// ============================================================================
// Constructors
// ============================================================================

Sequence::Sequence(std::string_view bases) {
    validateBases(bases);
    bases_.reserve(bases.length());
    std::ranges::transform(bases, std::back_inserter(bases_),
                          [](char c) { return toUpper(c); });
}

Sequence::Sequence(std::string_view bases, std::string id)
    : Sequence(bases) {
    id_ = std::move(id);
}

// ============================================================================
// Validation
// ============================================================================

constexpr bool Sequence::isValidBase(char c) noexcept {
    c = toUpper(c);
    return c == 'A' || c == 'C' || c == 'G' || c == 'T' || c == 'N';
}

bool Sequence::isValid() const noexcept {
    return std::ranges::all_of(bases_, [](char c) { return isValidBase(c); });
}

bool Sequence::hasAmbiguousBases() const noexcept {
    return std::ranges::any_of(bases_, [](char c) { return c == 'N'; });
}

void Sequence::validateBases(std::string_view bases) {
    if (bases.empty()) {
        throw SequenceError("Sequence cannot be empty");
    }

    for (size_t i = 0; i < bases.length(); ++i) {
        if (!isValidBase(bases[i])) {
            throw SequenceError("Invalid base '" + std::string(1, bases[i]) +
                              "' at position " + std::to_string(i));
        }
    }
}

// ============================================================================
// Content Analysis
// ============================================================================

double Sequence::gcContent() const noexcept {
    if (bases_.empty()) return 0.0;

    auto gc_count = std::ranges::count_if(bases_, [](char c) {
        return c == 'G' || c == 'C';
    });

    return static_cast<double>(gc_count) / static_cast<double>(bases_.length());
}

double Sequence::atContent() const noexcept {
    if (bases_.empty()) return 0.0;

    auto at_count = std::ranges::count_if(bases_, [](char c) {
        return c == 'A' || c == 'T';
    });

    return static_cast<double>(at_count) / static_cast<double>(bases_.length());
}

size_t Sequence::countBase(char base) const noexcept {
    base = toUpper(base);
    return static_cast<size_t>(std::ranges::count(bases_, base));
}

std::array<size_t, 5> Sequence::baseComposition() const noexcept {
    std::array<size_t, 5> counts{};  // A, C, G, T, N

    for (char c : bases_) {
        switch (c) {
            case 'A': counts[0]++; break;
            case 'C': counts[1]++; break;
            case 'G': counts[2]++; break;
            case 'T': counts[3]++; break;
            case 'N': counts[4]++; break;
        }
    }

    return counts;
}

// ============================================================================
// Transformations
// ============================================================================

constexpr char Sequence::toUpper(char c) noexcept {
    if (c >= 'a' && c <= 'z') {
        return static_cast<char>(c - 'a' + 'A');
    }
    return c;
}

constexpr char Sequence::complementBase(char c) noexcept {
    switch (c) {
        case 'A': return 'T';
        case 'T': return 'A';
        case 'C': return 'G';
        case 'G': return 'C';
        default: return 'N';
    }
}

Sequence Sequence::complement() const {
    std::string comp;
    comp.reserve(bases_.length());

    std::ranges::transform(bases_, std::back_inserter(comp), complementBase);

    // Use unchecked factory since we know the result is valid
    Sequence result = makeSequenceUnchecked(std::move(comp));
    result.id_ = id_;
    return result;
}

Sequence Sequence::reverseComplement() const {
    std::string rc;
    rc.reserve(bases_.length());

    // Transform and reverse in one pass using reverse iterators
    std::transform(bases_.rbegin(), bases_.rend(), std::back_inserter(rc),
                   complementBase);

    Sequence result = makeSequenceUnchecked(std::move(rc));
    result.id_ = id_;
    return result;
}

Sequence Sequence::reverse() const {
    std::string reversed(bases_.rbegin(), bases_.rend());

    Sequence result = makeSequenceUnchecked(std::move(reversed));
    result.id_ = id_;
    return result;
}

Sequence Sequence::subsequence(size_t start, size_t length) const {
    if (start >= bases_.length()) {
        throw SequenceError("Subsequence start position out of range");
    }

    auto actual_length = std::min(length, bases_.length() - start);
    auto sub = bases_.substr(start, actual_length);

    Sequence result = makeSequenceUnchecked(std::move(sub));
    if (id_) {
        result.id_ = *id_ + "_" + std::to_string(start) + "_" + std::to_string(actual_length);
    }
    return result;
}

Sequence Sequence::toUpperCase() const {
    // Already uppercase from constructor
    return *this;
}

// ============================================================================
// Motif Finding
// ============================================================================

bool Sequence::containsMotif(std::string_view motif) const {
    return bases_.find(motif) != std::string::npos;
}

std::vector<size_t> Sequence::findMotifPositions(std::string_view motif) const {
    std::vector<size_t> positions;

    if (motif.empty() || motif.length() > bases_.length()) {
        return positions;
    }

    size_t pos = 0;
    while ((pos = bases_.find(motif, pos)) != std::string::npos) {
        positions.push_back(pos);
        ++pos;  // Move past this match for overlapping matches
    }

    return positions;
}

size_t Sequence::countMotif(std::string_view motif) const {
    return findMotifPositions(motif).size();
}

// ============================================================================
// Operators
// ============================================================================

Sequence Sequence::operator+(const Sequence& other) const {
    std::string combined = bases_ + other.bases_;
    return makeSequenceUnchecked(std::move(combined));
}

std::string Sequence::toString() const {
    std::ostringstream oss;
    if (id_) {
        oss << ">" << *id_ << "\n";
    }
    oss << bases_;
    return oss.str();
}

// ============================================================================
// Factory Functions
// ============================================================================

Sequence makeSequence(std::string_view bases) {
    return Sequence(bases);
}

Sequence makeSequenceUnchecked(std::string bases) {
    // Private constructor bypass - create empty and swap
    // This is safe because we guarantee bases are already validated/uppercase
    Sequence seq("A");  // Dummy valid sequence
    seq.bases_ = std::move(bases);
    return seq;
}

// ============================================================================
// Stream Output
// ============================================================================

std::ostream& operator<<(std::ostream& os, const Sequence& seq) {
    if (seq.id()) {
        os << ">" << *seq.id() << "\n";
    }
    os << seq.bases();
    return os;
}

} // namespace bioflow
