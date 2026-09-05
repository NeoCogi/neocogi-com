# Workflow Examples

This directory contains multi-crate examples for recommended `nc-com`
integration workflows. They are ordinary Cargo workspaces and are also exercised
by integration tests so the examples stay buildable.

## Examples

- `contract-crate`: a host/plugin setup where both sides share a small `no_std`
  ABI contract crate.
