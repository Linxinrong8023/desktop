# Specification filesystem support

This module provides bounded Markdown discovery and safe specification reads within an already
resolved workspace root. It uses Ora's injected bundled ripgrep process, honors Git ignore rules for
discovery, and supports scans of built-in source directories with ignore rules disabled.

The module never decides project ownership or source classification. Canonical containment checks
reject traversal and symbolic-link escapes before paths cross the adapter boundary.
