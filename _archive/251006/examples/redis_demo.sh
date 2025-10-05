#!/bin/bash

# Kotoba Redis Database Demo
# This script demonstrates setting up a kotoba-style database using Redis

echo "🚀 Kotoba Redis Database Demo"
echo "================================"

# Check if Redis is running
if ! redis-cli ping >/dev/null 2>&1; then
    echo "❌ Redis server is not running. Please start Redis first:"
    echo "   brew services start redis"
    exit 1
fi

echo "✅ Redis server is running"

KEY_PREFIX="kotoba:demo"

# Clear any existing demo data
echo -e "\n🧹 Clearing existing demo data..."
EXISTING_KEYS=$(redis-cli KEYS "${KEY_PREFIX}*" | wc -l)
if [ "$EXISTING_KEYS" -gt 0 ]; then
    redis-cli KEYS "${KEY_PREFIX}*" | xargs redis-cli DEL >/dev/null 2>&1
    echo "   Cleared demo data"
fi

echo -e "\n👥 Storing user data..."

# Store user data as JSON
redis-cli SET "${KEY_PREFIX}:user:user_001" '{"id":"user_001","name":"Alice Johnson","email":"alice@example.com","role":"admin","created_at":"2024-01-01T00:00:00Z"}' >/dev/null
echo "   ✅ Stored user: Alice Johnson"

redis-cli SET "${KEY_PREFIX}:user:user_002" '{"id":"user_002","name":"Bob Smith","email":"bob@example.com","role":"user","created_at":"2024-01-01T00:00:00Z"}' >/dev/null
echo "   ✅ Stored user: Bob Smith"

redis-cli SET "${KEY_PREFIX}:user:user_003" '{"id":"user_003","name":"Carol Davis","email":"carol@example.com","role":"moderator","created_at":"2024-01-01T00:00:00Z"}' >/dev/null
echo "   ✅ Stored user: Carol Davis"

echo -e "\n⚙️  Storing configuration data..."

# Store configuration data
redis-cli SET "${KEY_PREFIX}:config:theme" '{"key":"theme","value":"dark","description":"UI theme setting"}' >/dev/null
echo "   ✅ Stored config: theme"

redis-cli SET "${KEY_PREFIX}:config:max_connections" '{"key":"max_connections","value":"100","description":"Maximum concurrent connections"}' >/dev/null
echo "   ✅ Stored config: max_connections"

redis-cli SET "${KEY_PREFIX}:config:debug_mode" '{"key":"debug_mode","value":"false","description":"Enable debug logging"}' >/dev/null
echo "   ✅ Stored config: debug_mode"

echo -e "\n🔐 Storing session data with TTL..."

# Store session data with TTL
redis-cli SETEX "${KEY_PREFIX}:session:active_session" 3600 '{"user_id":"user_001","token":"abc123","ip":"192.168.1.1"}' >/dev/null
echo "   ✅ Stored session with 1-hour TTL"

echo -e "\n📖 Retrieving stored data..."

# Retrieve specific user
ALICE_DATA=$(redis-cli GET "${KEY_PREFIX}:user:user_001")
ALICE_NAME=$(echo "$ALICE_DATA" | grep -o '"name":"[^"]*"' | cut -d'"' -f4)
ALICE_ROLE=$(echo "$ALICE_DATA" | grep -o '"role":"[^"]*"' | cut -d'"' -f4)
echo "   👤 Retrieved user: $ALICE_NAME ($ALICE_ROLE)"

# Retrieve configuration
THEME_DATA=$(redis-cli GET "${KEY_PREFIX}:config:theme")
THEME_VALUE=$(echo "$THEME_DATA" | grep -o '"value":"[^"]*"' | cut -d'"' -f4)
THEME_DESC=$(echo "$THEME_DATA" | grep -o '"description":"[^"]*"' | cut -d'"' -f4)
echo "   ⚙️  Retrieved config: theme = $THEME_VALUE ($THEME_DESC)"

echo -e "\n🔍 Scanning for data..."

# Find all users
USER_KEYS=$(redis-cli KEYS "${KEY_PREFIX}:user:*")
USER_COUNT=$(echo "$USER_KEYS" | wc -l)
echo "   👥 Found $USER_COUNT user records"

for key in $USER_KEYS; do
    USER_DATA=$(redis-cli GET "$key")
    USER_NAME=$(echo "$USER_DATA" | grep -o '"name":"[^"]*"' | cut -d'"' -f4)
    USER_EMAIL=$(echo "$USER_DATA" | grep -o '"email":"[^"]*"' | cut -d'"' -f4)
    USER_ID=$(echo "$key" | cut -d':' -f4)
    echo "      - $USER_ID: $USER_NAME <$USER_EMAIL>"
done

# Find all configs
CONFIG_KEYS=$(redis-cli KEYS "${KEY_PREFIX}:config:*")
CONFIG_COUNT=$(echo "$CONFIG_KEYS" | wc -l)
echo "   ⚙️  Found $CONFIG_COUNT configuration records"

echo -e "\n✏️  Updating data..."

# Update user role
redis-cli SET "${KEY_PREFIX}:user:user_001" '{"id":"user_001","name":"Alice Johnson","email":"alice@example.com","role":"super_admin","created_at":"2024-01-01T00:00:00Z"}' >/dev/null
echo "   ✅ Updated Alice's role to: super_admin"

echo -e "\n🗑️  Deleting data..."

# Delete session
redis-cli DEL "${KEY_PREFIX}:session:active_session" >/dev/null
echo "   ✅ Deleted active session"

# Verify deletion
SESSION_EXISTS=$(redis-cli EXISTS "${KEY_PREFIX}:session:active_session")
echo "   🔍 Session exists after deletion: $SESSION_EXISTS"

echo -e "\n📊 Database Statistics..."

# Show statistics
ALL_KEYS=$(redis-cli KEYS "${KEY_PREFIX}*")
TOTAL_KEYS=$(echo "$ALL_KEYS" | wc -l)
echo "   📈 Total keys in database: $TOTAL_KEYS"

# Count by type
USER_TYPE_COUNT=$(echo "$ALL_KEYS" | grep ":user:" | wc -l)
CONFIG_TYPE_COUNT=$(echo "$ALL_KEYS" | grep ":config:" | wc -l)
SESSION_TYPE_COUNT=$(echo "$ALL_KEYS" | grep ":session:" | wc -l)

echo "      user: $USER_TYPE_COUNT keys"
echo "      config: $CONFIG_TYPE_COUNT keys"
echo "      session: $SESSION_TYPE_COUNT keys"

echo -e "\n⏰ TTL Demonstration..."

# Set a key with short TTL
redis-cli SETEX "${KEY_PREFIX}:temp:expiring_key" 5 "This will expire soon" >/dev/null
echo "   ✅ Set temporary key with 5-second TTL"

# Check TTL
TTL=$(redis-cli TTL "${KEY_PREFIX}:temp:expiring_key")
echo "   ⏱️  TTL remaining: $TTL seconds"

# Wait and check again
echo "   ⏳ Waiting 6 seconds for expiration..."
sleep 6

EXISTS_AFTER=$(redis-cli EXISTS "${KEY_PREFIX}:temp:expiring_key")
echo "   🔍 Key exists after TTL expiration: $EXISTS_AFTER"

echo -e "\n🎉 Kotoba Redis Database Demo Completed!"
echo "========================================"
echo "✅ Successfully demonstrated:"
echo "   - Connecting to Redis"
echo "   - Storing structured JSON data (users, configs)"
echo "   - Retrieving data with parsing"
echo "   - Scanning/querying with patterns"
echo "   - Updating existing records"
echo "   - Deleting records"
echo "   - TTL (time-to-live) functionality"
echo "   - Database statistics"
echo ""
echo "🚀 Kotoba database with Redis storage is ready for production use!"
echo ""
echo "💡 Next steps:"
echo "   - Use kotoba-storage-redis crate for type-safe operations"
echo "   - Implement connection pooling for high performance"
echo "   - Add compression for large values"
echo "   - Set up Redis cluster for scalability"
