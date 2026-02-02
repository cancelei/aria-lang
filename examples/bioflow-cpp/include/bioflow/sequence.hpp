#pragma once

#include <string>
#include <string_view>
#include <optional>
#include <stdexcept>
#include <algorithm>
#include <ranges>
#include <vector>
#include <concepts>

namespace bioflow {

/**
 * @brief Exception class for sequence-related errors
 */
class SequenceError : public std::runtime_error {
public:
    using std::runtime_error::runtime_error;
};

/**
 * @brief C++20 concept for sequence-like types
 */
template<typename T>
concept SequenceLike = requires(T t) {
    { t.bases() } -> std::convertible_to<std::string_view>;
    { t.length() } -> std::convertible_to<size_t>;
};

/**
 * @brief Represents a DNA sequence with validation and analysis capabilities
 *
 * This class provides a type-safe representation of DNA sequences with
 * automatic validation, content analysis, and transformation operations.
 * Uses modern C++20 features including ranges, concepts, and constexpr.
 */
class Sequence {
public:
    // Constructors
    explicit Sequence(std::string_view bases);
    Sequence(std::string_view bases, std::string id);

    // Default special members
    Sequence(const Sequence&) = default;
    Sequence(Sequence&&) noexcept = default;
    Sequence& operator=(const Sequence&) = default;
    Sequence& operator=(Sequence&&) noexcept = default;
    ~Sequence() = default;

    // Getters
    [[nodiscard]] const std::string& bases() const noexcept { return bases_; }
    [[nodiscard]] size_t length() const noexcept { return bases_.length(); }
    [[nodiscard]] const std::optional<std::string>& id() const noexcept { return id_; }
    [[nodiscard]] bool empty() const noexcept { return bases_.empty(); }

    // Iterator support for ranges
    [[nodiscard]] auto begin() const noexcept { return bases_.begin(); }
    [[nodiscard]] auto end() const noexcept { return bases_.end(); }
    [[nodiscard]] auto cbegin() const noexcept { return bases_.cbegin(); }
    [[nodiscard]] auto cend() const noexcept { return bases_.cend(); }

    // Element access
    [[nodiscard]] char operator[](size_t index) const { return bases_[index]; }
    [[nodiscard]] char at(size_t index) const { return bases_.at(index); }

    // Validation
    [[nodiscard]] static constexpr bool isValidBase(char c) noexcept;
    [[nodiscard]] bool isValid() const noexcept;
    [[nodiscard]] bool hasAmbiguousBases() const noexcept;

    // Content analysis
    [[nodiscard]] double gcContent() const noexcept;
    [[nodiscard]] double atContent() const noexcept;
    [[nodiscard]] size_t countBase(char base) const noexcept;
    [[nodiscard]] std::array<size_t, 5> baseComposition() const noexcept;

    // Transformations - return new sequences (immutable design)
    [[nodiscard]] Sequence complement() const;
    [[nodiscard]] Sequence reverseComplement() const;
    [[nodiscard]] Sequence reverse() const;
    [[nodiscard]] Sequence subsequence(size_t start, size_t length) const;
    [[nodiscard]] Sequence toUpperCase() const;

    // Motif finding
    [[nodiscard]] bool containsMotif(std::string_view motif) const;
    [[nodiscard]] std::vector<size_t> findMotifPositions(std::string_view motif) const;
    [[nodiscard]] size_t countMotif(std::string_view motif) const;

    // Operators
    [[nodiscard]] bool operator==(const Sequence& other) const = default;
    [[nodiscard]] auto operator<=>(const Sequence& other) const = default;

    // Concatenation
    [[nodiscard]] Sequence operator+(const Sequence& other) const;

    // String conversion
    [[nodiscard]] std::string toString() const;

private:
    std::string bases_;
    std::optional<std::string> id_;

    static void validateBases(std::string_view bases);
    [[nodiscard]] static constexpr char toUpper(char c) noexcept;
    [[nodiscard]] static constexpr char complementBase(char c) noexcept;
};

// Factory functions
[[nodiscard]] Sequence makeSequence(std::string_view bases);
[[nodiscard]] Sequence makeSequenceUnchecked(std::string bases);

// Stream output
std::ostream& operator<<(std::ostream& os, const Sequence& seq);

} // namespace bioflow
