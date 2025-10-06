{ pkgs ? import <nixpkgs> {}, rustToolchain }:

pkgs.mkShell rec {
  name = "kanban-rust-shell";

  PGHOST = "localhost";
  PGPORT = 5432;
  PGUSER = "kanban";
  PGPASSWORD = "kanban_dev";
  PGDATABASE = "kanban_dev";

  DATABASE_URL = "postgresql://${PGUSER}:${PGPASSWORD}@${PGHOST}:${toString PGPORT}/${PGDATABASE}";

  buildInputs = with pkgs; [
    # Rust toolchain
    rustToolchain
    cargo-watch
    cargo-edit
    cargo-audit
    cargo-tarpaulin

    # Build dependencies
    pkg-config
    openssl

    # Database tools
    pgcli
    postgresql_15
    diesel-cli

    # Development utilities
    bacon

    # PostgreSQL management scripts
    (pkgs.writeScriptBin "pg-stop" ''
      echo "Stopping PostgreSQL server..."
      pg_ctl -D $PWD/.pgdata stop
    '')

    (pkgs.writeScriptBin "pg-start" ''
      export PGDATA=$PWD/.pgdata
      export PATH=$PATH:${pkgs.postgresql_15}/bin

      echo "Starting PostgreSQL server..."

      if [ ! -d "$PGDATA" ]; then
        echo "Initializing PostgreSQL data directory..."
        initdb -D "$PGDATA"
        echo "port = ${toString PGPORT}" >> "$PGDATA/postgresql.conf"
        echo "unix_socket_directories = '$PGDATA/sockets'" >> "$PGDATA/postgresql.conf"
      fi

      mkdir -p "$PGDATA/sockets"

      if pg_ctl -D "$PGDATA" -l "$PGDATA/logfile" start; then
        echo "Creating '${PGUSER}' role if it doesn't exist..."
        psql -U $USER -d postgres -c "
        DO \$\$
        BEGIN
          IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = '${PGUSER}') THEN
            CREATE ROLE ${PGUSER} WITH LOGIN PASSWORD '${PGPASSWORD}' CREATEDB;
          END IF;
        END
        \$\$;" || echo "Warning: Could not ensure '${PGUSER}' role"

        echo "Creating '${PGDATABASE}' database if it doesn't exist..."
        createdb -U $USER ${PGDATABASE} 2>/dev/null || echo "Database '${PGDATABASE}' already exists."

        echo "Granting privileges on '${PGDATABASE}' to '${PGUSER}'..."
        psql -U $USER -d ${PGDATABASE} -c "GRANT ALL PRIVILEGES ON DATABASE ${PGDATABASE} TO ${PGUSER};" || true

        echo "Granting schema privileges to '${PGUSER}'..."
        psql -U $USER -d ${PGDATABASE} -c "
          GRANT CREATE ON SCHEMA public TO ${PGUSER};
          GRANT USAGE ON SCHEMA public TO ${PGUSER};
        " || true

        echo
        echo "✅ PostgreSQL is running."
        echo "🔗 Connect with: psql or pgcli"
        echo "🛑 Stop with:    pg-stop"
      else
        echo
        echo "❌ PostgreSQL is **not** running."
        echo "Check the logs at '$PGDATA/logfile' for details."
      fi
    '')
  ];

  shellHook = ''
    export RUST_BACKTRACE=1
    export DATABASE_URL="${DATABASE_URL}"
    echo "🦀 Rust Kanban Development Environment"
    echo "📦 Cargo: $(cargo --version)"
    echo "🦀 Rustc: $(rustc --version)"
    echo "🗄️  Database: ${DATABASE_URL}"
  '';
}

