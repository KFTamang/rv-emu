#!/bin/bash

# Input log file
LOG_FILE="log/output_kernel.log"

# Database file
DB_FILE="emulator_logs.db"

# Reset the SQLite database and optimize settings
sqlite3 "$DB_FILE" <<EOF
PRAGMA journal_mode=WAL;           -- Enable faster concurrent writes
PRAGMA synchronous=OFF;            -- Disable sync for better performance
DROP TABLE IF EXISTS logs;         -- Reset the table (delete all existing logs)
CREATE TABLE logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    level TEXT NOT NULL,
    module TEXT NOT NULL,
    message TEXT NOT NULL
);
EOF

# Buffer to batch inserts (for performance)
BUFFER_SIZE=1000
BUFFER=""
LINE_NUM=0

# Process the log file
while IFS= read -r line; do

    ((LINE_NUM++))

    # Skip the first 49 lines
    if (( LINE_NUM <= 49 )); then
        continue
    fi


    # Parse log fields
    timestamp=$(echo "$line" | awk '{print substr($1, 2, length($1) - 2)}')
    level=$(echo "$line" | awk '{print $2}')
    module=$(echo "$line" | awk '{print substr($3, 1, length($3) - 1)}')
    message=$(echo "$line" | sed -E 's/.*\] //')

    # Validate parsed fields
    if [[ -n "$timestamp" && -n "$level" && -n "$module" && -n "$message" ]]; then
        # Add to buffer
        # BUFFER+=$(echo "INSERT INTO logs (timestamp, level, module, message) VALUES ($timestamp, $level, $module, $message);")
        # echo "INSERT INTO logs (timestamp, level, module, message) VALUES ($timestamp, $level, $module, $message);"
        sqlite3 emulator_logs.db "INSERT INTO logs (timestamp, level, module, message) VALUES ('$timestamp', '$level', '$module', '$message');"
        # echo $BUFFER | wc -l
    fi

    # # Flush to the database when buffer is full
    # if [[ $(echo "$BUFFER" | wc -l) -ge $BUFFER_SIZE ]]; then
    #     echo "BEGIN TRANSACTION;" > batch.sql
    #     echo "$BUFFER" >> batch.sql
    #     echo "COMMIT;" >> batch.sql
    #     sqlite3 -batch "$DB_FILE" < batch.sql
    #     BUFFER=""
    # fi
done < "$LOG_FILE"

# Insert remaining logs
if [[ -n "$BUFFER" ]]; then
    echo "BEGIN TRANSACTION;" > batch.sql
    echo "$BUFFER" >> batch.sql
    echo "COMMIT;" >> batch.sql
    sqlite3 -batch "$DB_FILE" < batch.sql
fi

echo "✅ Log file imported into $DB_FILE"
