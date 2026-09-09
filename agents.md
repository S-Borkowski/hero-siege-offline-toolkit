# Agent Guidelines

## Submodule & Directory Development Instructions

When developing, modifying, testing, or investigating code within any submodule or specific directory (e.g., `HSCraftSim/`, `ForgePact/`, `HS-Offline-Tracker/`, etc.), always consult and follow the corresponding development instructions guide:
- Check the centralized index at [`docs/submodules/README.md`](docs/submodules/README.md) for available module guides.
- Check for submodule-specific development guides located at `docs/submodules/<submodule-name>/instructions.md` (or `<submodule-name>/instructions.md` if present within the directory).
- Adhere to the documented architecture, entry points, workflows, testing procedures, dependencies, and command conventions outlined in the relevant `instructions.md`.

## Documentation & Instructions Maintenance

Upon completing any task or making changes to features, workflows, architecture, or dependencies:
- Update documentation, instructions (such as submodule `instructions.md` files), and `README.md` files when and where relevant to reflect the changes.
- Ensure any new guides, updated links, or modified commands remain accurate and in sync across project and submodule documentation.

## YYToolkit Integration

When a prompt or task requires the use of `yytoolkit`, attempt to retrieve `yytoolkit` documentation and references from the `context7` MCP server if it is available.
