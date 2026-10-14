# Shared UI libraries

`ui-kit` is the shared frontend package. It builds an ESM package and a
Module Federation remote and owns the common Button, Badge, and Card primitives,
status classes, and their CSS tokens. Hosts and remotes should consume shared
primitives from this package rather than cloning their markup or styles.

Add another library only when code is actually shared across frontend apps or
needs a separately owned build/test boundary.
