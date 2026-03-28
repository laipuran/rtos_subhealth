module.exports = {
  extends: ["@commitlint/config-conventional"],
  rules: {
    "scope-case": [2, "always", "lower-case"],
    "scope-empty": [0],
    "scope-enum": [0],
    // Allow Chinese or mixed-case subjects.
    "subject-case": [0],
    "header-max-length": [2, "always", 120]
  }
};