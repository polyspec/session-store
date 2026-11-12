
- The repository depends on `polyspec-binding-core` of the repository binding (`binding_core`, the Git dependency
  `https://github.com/polyspec/binding.git`, which `Cargo.lock` pins), the new name of bindgen, and runs its tool with
  `scripts/binding.mts`; the binding packages are those that `binding scaffold` writes. The crate
  `polyspec-session-store` depends on `polyspec-php-serialization`, whose `php_value` its contract names, and
  `Cargo.lock` takes the commit of `polyspec-php-serialization` on the same `binding_core`. The error classes stay
  `SessionStore<Category>Error` in every language: the repository name has no acronym word (design N-2).
