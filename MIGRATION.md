# Project Migration Tracker

Migrating all projects from legacy single-volume layout to bind-mount + home-volume layout.

## Status

| Project      | Status      | Notes                          |
|--------------|-------------|--------------------------------|
| advice-cloud | ✅ Done      | Already migrated               |
| kyc          | ✅ Done      | Migrated 2026-04-20            |
| plotzy       | ✅ Done      | Migrated 2026-04-20            |
| jstockdi     | ✅ Done      | Migrated 2026-04-20            |

## Steps per project

1. `claudine build <project>` — rebuild image with latest source
2. `claudine migrate <project> --yes` — copy volume → host dir + home volume
3. Destroy old container: `claudine destroy <project>`
4. Verify with `claudine run <project>`
5. Clean up legacy volume: `docker volume rm claudine_<project>`
