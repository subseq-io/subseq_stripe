When using sqlx::QueryBuilder::separated for SET field = $N updates, use push_bind_unseparated for the bind value to avoid invalid SQL separator injection.
