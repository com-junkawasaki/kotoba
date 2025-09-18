# KotobaDB Query Language

KotobaDB implements a powerful graph-native query language that combines the best of Cypher (Neo4j), GraphQL, and SQL. This document provides comprehensive reference for writing queries in KotobaDB.

## Overview

The KotobaDB Query Language (KQL) is designed for:
- **Graph Traversal**: Navigate complex relationships
- **Pattern Matching**: Find data patterns in the graph
- **Aggregation**: Perform analytical queries
- **Mutations**: Create, update, and delete data
- **Transactions**: Atomic multi-step operations

## Basic Syntax

### Nodes and Relationships

```cypher
// Node syntax
(node_variable:NodeType {property: value})

// Relationship syntax
-[relationship_variable:RelationshipType {property: value}]-

// Directed relationships
-(from_node)-[relationship]->(to_node)
```

### Example Patterns

```cypher
// Simple node
(u:User)

// Node with properties
(u:User {name: "Alice", age: 30})

// Relationship
(u:User)-[f:FOLLOWS]->(u2:User)

// Complex pattern
(u:User)-[:POSTED]->(p:Post)<-[:COMMENTED]-(c:Comment)
```

## Query Types

### MATCH - Pattern Matching

Find data that matches specific patterns.

```cypher
// Basic node matching
MATCH (u:User)
RETURN u

// Property matching
MATCH (u:User {active: true})
RETURN u.name, u.email

// Relationship matching
MATCH (u:User)-[:FOLLOWS]->(friend:User)
RETURN u.name, friend.name

// Multiple relationships
MATCH (u:User)-[:POSTED]->(p:Post)<-[:LIKES]-(liker:User)
RETURN u.name, p.title, liker.name

// Variable-length paths
MATCH (u:User)-[:FOLLOWS*1..3]->(friend:User)
RETURN u.name, friend.name, length(path)
```

### CREATE - Data Creation

Create new nodes and relationships.

```cypher
// Create single node
CREATE (u:User {name: "Alice", email: "alice@example.com"})
RETURN u

// Create multiple nodes
CREATE (u1:User {name: "Bob"}), (u2:User {name: "Charlie"})
RETURN u1, u2

// Create nodes with relationships
CREATE (u:User {name: "Alice"})-[:FOLLOWS]->(u2:User {name: "Bob"})
RETURN u, u2

// Create complex graph structures
CREATE (u:User {name: "Alice"})
CREATE (p:Post {title: "Hello World", content: "My first post"})
CREATE (u)-[:POSTED {timestamp: datetime()}]->(p)
RETURN u, p
```

### MERGE - Conditional Creation

Create data if it doesn't exist, match if it does.

```cypher
// Merge node by properties
MERGE (u:User {email: "alice@example.com"})
ON CREATE SET u.name = "Alice", u.created_at = datetime()
ON MATCH SET u.last_login = datetime()
RETURN u

// Merge relationships
MERGE (u:User {name: "Alice"})
MERGE (p:Post {title: "Hello"})
MERGE (u)-[r:POSTED]->(p)
ON CREATE SET r.created_at = datetime()
RETURN u, p, r
```

### UPDATE - Data Modification

Update existing nodes and relationships.

```cypher
// Update node properties
MATCH (u:User {name: "Alice"})
SET u.age = 31, u.updated_at = datetime()
RETURN u

// Update multiple properties
MATCH (u:User)
WHERE u.last_login < datetime() - duration('P30D')
SET u.status = "inactive"
RETURN count(u)

// Update relationships
MATCH (u:User)-[r:FOLLOWS]->(friend:User)
WHERE r.since < datetime() - duration('P1Y')
SET r.strength = "weak"
RETURN r

// Remove properties
MATCH (u:User {name: "Alice"})
REMOVE u.temp_field
RETURN u
```

### DELETE - Data Removal

Delete nodes and relationships.

```cypher
// Delete single node
MATCH (u:User {name: "Alice"})
DELETE u

// Delete with relationships
MATCH (u:User {name: "Alice"})
OPTIONAL MATCH (u)-[r]-()
DELETE r, u

// Delete relationships only
MATCH (u:User)-[r:FOLLOWS]->(friend:User)
WHERE r.since < datetime() - duration('P2Y')
DELETE r

// Detach delete (removes all relationships automatically)
MATCH (u:User {inactive: true})
DETACH DELETE u
```

## Advanced Patterns

### Path Variables

Capture entire paths in queries.

```cypher
// Store path in variable
MATCH path = (u:User)-[:FOLLOWS*1..5]->(friend:User)
WHERE u.name = "Alice"
RETURN path, length(path)

// Path properties
MATCH path = (start:User)-[rels:FOLLOWS*]->(end:User)
WHERE start.name = "Alice" AND end.name = "Charlie"
RETURN path, nodes(path), relationships(path), length(path)
```

### List Comprehensions

Process collections of data.

```cypher
// Filter and transform lists
MATCH (u:User)
RETURN u.name, [tag IN u.tags WHERE tag STARTS WITH 'tech'] AS tech_tags

// Collect and aggregate
MATCH (u:User)-[:POSTED]->(p:Post)
RETURN u.name, collect(p.title) AS posts, count(p) AS post_count

// List operations
MATCH (u:User)
RETURN u.name,
       size(u.tags) AS tag_count,
       u.tags[0..5] AS first_five_tags
```

### Temporal Queries

Work with time-based data.

```cypher
// Time range queries
MATCH (p:Post)
WHERE p.created_at >= datetime('2024-01-01T00:00:00Z')
  AND p.created_at < datetime('2024-02-01T00:00:00Z')
RETURN p

// Relative time queries
MATCH (u:User)
WHERE u.last_login > datetime() - duration('P7D')
RETURN u

// Time aggregations
MATCH (p:Post)
RETURN date(p.created_at) AS day, count(p) AS posts_per_day
ORDER BY day
```

### Full-Text Search

Text search capabilities.

```cypher
// Full-text search on content
MATCH (p:Post)
WHERE p.content CONTAINS 'database'
RETURN p.title, p.content

// Fuzzy search
MATCH (u:User)
WHERE u.name =~ '(?i).*alice.*'
RETURN u.name

// Regex matching
MATCH (p:Post)
WHERE p.title =~ 'Tutorial.*'
RETURN p.title
```

## Aggregation and Analytics

### Basic Aggregation

```cypher
// Count operations
MATCH (u:User)
RETURN count(u) AS user_count

// Group by operations
MATCH (u:User)-[:POSTED]->(p:Post)
RETURN u.name, count(p) AS post_count
ORDER BY post_count DESC

// Multiple aggregations
MATCH (p:Post)
RETURN count(p) AS total_posts,
       avg(p.views) AS avg_views,
       max(p.likes) AS max_likes,
       min(p.created_at) AS oldest_post
```

### Advanced Analytics

```cypher
// Statistical functions
MATCH (p:Post)
RETURN stdev(p.views) AS view_stddev,
       percentile(p.views, 0.95) AS p95_views

// Window functions
MATCH (u:User)-[:POSTED]->(p:Post)
RETURN u.name, p.title, p.views,
       rank() OVER (ORDER BY p.views DESC) AS popularity_rank

// Graph analytics
MATCH (u:User)
RETURN u.name,
       size((u)-[:FOLLOWS]->()) AS following_count,
       size((u)<-[:FOLLOWS]-()) AS follower_count

// Path analytics
MATCH path = shortestPath((u1:User)-[*]-(u2:User))
WHERE u1.name = "Alice" AND u2.name = "Charlie"
RETURN path, length(path) AS degrees_of_separation
```

## Subqueries and CTEs

### Common Table Expressions

```cypher
// Define CTE
WITH user_posts AS (
  MATCH (u:User)-[:POSTED]->(p:Post)
  RETURN u.name AS user_name, p.title AS post_title, p.views
)
MATCH (up:user_posts)
WHERE up.views > 1000
RETURN up.user_name, collect(up.post_title) AS popular_posts
```

### Subqueries

```cypher
// EXISTS subquery
MATCH (u:User)
WHERE EXISTS {
  MATCH (u)-[:POSTED]->(p:Post)
  WHERE p.views > 1000
}
RETURN u.name

// COUNT subquery
MATCH (u:User)
RETURN u.name,
       COUNT { MATCH (u)-[:POSTED]->(p:Post) } AS post_count,
       COUNT { MATCH (u)-[:LIKES]->(p:Post) } AS like_count
```

## Transaction Control

### Explicit Transactions

```cypher
// Begin transaction
BEGIN

// Execute operations
CREATE (u:User {name: "Alice", email: "alice@example.com"})
CREATE (p:Post {title: "Hello", content: "World"})
CREATE (u)-[:POSTED]->(p)

// Commit or rollback
COMMIT
// or ROLLBACK
```

### Transaction Modes

```cypher
// Read-only transaction
BEGIN READ ONLY
MATCH (u:User) RETURN u
COMMIT

// Serializable transaction
BEGIN ISOLATION LEVEL SERIALIZABLE
MATCH (u:User {name: "Alice"})
SET u.balance = u.balance - 100
COMMIT
```

## Indexing and Performance

### Index Hints

```cypher
// Force index usage
MATCH (u:User)
USING INDEX u:User(name)
WHERE u.name = "Alice"
RETURN u

// Multiple indexes
MATCH (u:User)-[:POSTED]->(p:Post)
USING INDEX u:User(email)
USING INDEX p:Post(created_at)
WHERE u.email = "alice@example.com" AND p.created_at > datetime('2024-01-01')
RETURN u, p
```

### Query Profiling

```cypher
// Profile query execution
PROFILE MATCH (u:User)-[:FOLLOWS*1..3]->(friend:User)
WHERE u.name = "Alice"
RETURN u, friend

// Explain query plan
EXPLAIN MATCH (u:User {active: true})
RETURN u.name, u.email
```

## Schema and Constraints

### Schema-Aware Queries

```cypher
// Query with schema validation
MATCH (u:User {name: $name})
WHERE u IS VALID  // Validate against schema
RETURN u

// Schema-aware creation
CREATE (u:User) {
  name: "Alice",
  email: "alice@example.com",
  age: 30
} VALIDATE  // Validate properties against schema
RETURN u
```

## Error Handling

### Try-Catch Blocks

```cypher
TRY {
  MATCH (u:User {id: $user_id})
  SET u.balance = u.balance - $amount
  RETURN "success"
} CATCH (error) {
  RETURN "error: " + error.message
}
```

## Extensions and Plugins

### Custom Functions

```cypher
// Use custom functions
RETURN custom.distance($lat1, $lon1, $lat2, $lon2) AS distance

// Plugin functions
RETURN plugin.ml.predict($features) AS prediction
```

### Stored Procedures

```cypher
// Call stored procedure
CALL user_management.create_user($name, $email, $password)
YIELD user_id, status
RETURN user_id, status
```

## Best Practices

### Query Optimization

1. **Use specific labels and properties**
   ```cypher
   // Good
   MATCH (u:User {active: true})
   RETURN u.name

   // Avoid
   MATCH (u)
   WHERE u.active = true AND labels(u) = ['User']
   RETURN u.name
   ```

2. **Leverage indexes**
   ```cypher
   // Indexed property first
   MATCH (u:User {email: $email})
   MATCH (u)-[:POSTED]->(p:Post)
   RETURN p
   ```

3. **Use appropriate data types**
   ```cypher
   // Use datetime for time comparisons
   MATCH (p:Post)
   WHERE p.created_at >= datetime('2024-01-01')
   RETURN p
   ```

### Performance Patterns

1. **Batch operations**
   ```cypher
   // Instead of multiple queries
   UNWIND $user_data AS user
   CREATE (u:User) SET u = user
   ```

2. **Use path variables wisely**
   ```cypher
   // Limit path length for performance
   MATCH path = (u:User)-[:FOLLOWS*1..3]->(friend:User)
   RETURN path
   ```

3. **Profile before optimizing**
   ```cypher
   PROFILE MATCH (u:User)-[:FOLLOWS]->(f:User)
   RETURN u, f
   ```

## Migration from Other Languages

### From SQL

```sql
-- SQL
SELECT u.name, COUNT(p.id) as post_count
FROM users u
LEFT JOIN posts p ON u.id = p.user_id
GROUP BY u.id, u.name
HAVING COUNT(p.id) > 0

-- KotobaDB
MATCH (u:User)
OPTIONAL MATCH (u)-[:POSTED]->(p:Post)
RETURN u.name, count(p) as post_count
ORDER BY post_count DESC
```

### From Cypher (Neo4j)

```cypher
// Neo4j Cypher (most syntax works directly)
MATCH (u:User)-[:FOLLOWS]->(friend:User)
RETURN u.name, friend.name

// KotobaDB supports the same syntax
MATCH (u:User)-[:FOLLOWS]->(friend:User)
RETURN u.name, friend.name
```

### From GraphQL

```graphql
# GraphQL
{
  users {
    name
    posts {
      title
      comments {
        author {
          name
        }
      }
    }
  }
}

# Equivalent KotobaDB query
MATCH (u:User)-[:POSTED]->(p:Post)<-[:COMMENTED]-(c:Comment)<-[:AUTHOR]-(commenter:User)
RETURN u.name, p.title, commenter.name
```

This query language reference provides the foundation for working with KotobaDB. The language is designed to be intuitive for developers familiar with graph databases while providing powerful features for complex analytical queries.
