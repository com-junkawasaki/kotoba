// Database Configuration Library
// データベース設定用のJsonnetライブラリ

{
  // データベースタイプ
  types: {
    String: { max_length: null },
    Text: 'Text',
    Integer: 'Integer',
    BigInt: 'BigInt',
    Float: 'Float',
    Double: 'Double',
    Decimal: { precision: 10, scale: 2 },
    Boolean: 'Boolean',
    Date: 'Date',
    DateTime: 'DateTime',
    Time: 'Time',
    UUID: 'UUID',
    JSON: 'JSON',
    Binary: 'Binary',
  },

  // PostgreSQL設定
  postgres:: {
    db_type: 'PostgreSQL',

    // モデル定義ヘルパー
    model:: {
      table_name: error 'table_name must be specified',
      fields: [],
      relationships: [],
      indexes: [],
    },

    // フィールド定義ヘルパー
    field:: {
      name: error 'name must be specified',
      field_type: error 'field_type must be specified',
      nullable: false,
      default_value: null,
      unique: false,
      primary_key: false,
    },

    // リレーションシップ定義ヘルパー
    relationship:: {
      name: error 'name must be specified',
      target_model: error 'target_model must be specified',
      relationship_type: 'OneToMany',
      foreign_key: error 'foreign_key must be specified',
      on_delete: 'Restrict',
      on_update: 'NoAction',
    },

    // インデックス定義ヘルパー
    index:: {
      name: error 'name must be specified',
      fields: error 'fields must be specified',
      unique: false,
      index_type: 'BTree',
    },

    // マイグレーション定義ヘルパー
    migration:: {
      version: error 'version must be specified',
      description: error 'description must be specified',
      up_sql: error 'up_sql must be specified',
      down_sql: error 'down_sql must be specified',
      dependencies: [],
    },
  },

  // MySQL設定
  mysql:: {
    db_type: 'MySQL',
  } + $.postgres,

  // SQLite設定
  sqlite:: {
    db_type: 'SQLite',
  } + $.postgres,

  // MongoDB設定
  mongodb:: {
    db_type: 'MongoDB',
  },

  // Redis設定
  redis:: {
    db_type: 'Redis',
  },

  // 便利なフィールドタイプ関数
  field: {
    id(name='id'):: $.postgres.field {
      name: name,
      field_type: $.types.UUID,
      primary_key: true,
      nullable: false,
    },

    string(name, max_length=255):: $.postgres.field {
      name: name,
      field_type: $.types.String { max_length: max_length },
    },

    text(name):: $.postgres.field {
      name: name,
      field_type: $.types.Text,
    },

    integer(name):: $.postgres.field {
      name: name,
      field_type: $.types.Integer,
    },

    boolean(name):: $.postgres.field {
      name: name,
      field_type: $.types.Boolean,
    },

    datetime(name):: $.postgres.field {
      name: name,
      field_type: $.types.DateTime,
    },

    timestamps():: [
      $.field.datetime('created_at') {
        nullable: false,
        default_value: 'NOW()',
      },
      $.field.datetime('updated_at') {
        nullable: false,
        default_value: 'NOW()',
      },
    ],

    soft_delete():: [
      $.field.datetime('deleted_at') {
        nullable: true,
      },
    ],
  },

  // 便利なリレーションシップ関数
  relationship: {
    belongsTo(target_model, foreign_key):: $.postgres.relationship {
      target_model: target_model,
      relationship_type: 'ManyToOne',
      foreign_key: foreign_key,
    },

    hasMany(target_model, foreign_key):: $.postgres.relationship {
      target_model: target_model,
      relationship_type: 'OneToMany',
      foreign_key: foreign_key,
    },

    hasOne(target_model, foreign_key):: $.postgres.relationship {
      target_model: target_model,
      relationship_type: 'OneToOne',
      foreign_key: foreign_key,
    },

    belongsToMany(target_model, join_table):: $.postgres.relationship {
      target_model: target_model,
      relationship_type: 'ManyToMany',
      foreign_key: join_table,
    },
  },

  // 便利なインデックス関数
  index: {
    unique(name, fields):: $.postgres.index {
      name: name,
      fields: if std.isArray(fields) then fields else [fields],
      unique: true,
    },

    composite(name, fields):: $.postgres.index {
      name: name,
      fields: if std.isArray(fields) then fields else [fields],
    },

    gin(name, field):: $.postgres.index {
      name: name,
      fields: [field],
      index_type: 'GIN',
    },

    gist(name, field):: $.postgres.index {
      name: name,
      fields: [field],
      index_type: 'GiST',
    },
  },

  // マイグレーションヘルパー
  migration: {
    createTable(table_name, fields):: $.postgres.migration {
      up_sql: std.join('\n', [
        'CREATE TABLE %s (' % table_name,
        std.join(',\n', [
          local formatField(field) =
            local typeStr = if std.isObject(field.field_type) then
              if field.field_type.max_length != null then
                'VARCHAR(%d)' % field.field_type.max_length
              else
                std.asciiUpper(field.field_type)
            else
              std.asciiUpper(field.field_type);
            local nullable = if field.nullable then '' else ' NOT NULL';
            local default = if field.default_value != null then ' DEFAULT ' + field.default_value else '';
            local unique = if field.unique then ' UNIQUE' else '';
            local pk = if field.primary_key then ' PRIMARY KEY' else '';
            '  %s %s%s%s%s%s' % [field.name, typeStr, nullable, default, unique, pk];
          formatField(field) for field in fields
        ]),
        ');',
      ]),

      down_sql: 'DROP TABLE %s;' % table_name,
    },

    addIndex(table_name, index):: $.postgres.migration {
      local unique = if index.unique then 'UNIQUE ' else '';
      local fields = std.join(', ', index.fields);

      up_sql: 'CREATE %sINDEX %s ON %s (%s);' % [unique, index.name, table_name, fields],
      down_sql: 'DROP INDEX %s;' % index.name,
    },

    addColumn(table_name, field):: $.postgres.migration {
      local typeStr = if std.isObject(field.field_type) then
        if field.field_type.max_length != null then
          'VARCHAR(%d)' % field.field_type.max_length
        else
          std.asciiUpper(field.field_type)
      else
        std.asciiUpper(field.field_type);
      local nullable = if field.nullable then '' else ' NOT NULL';
      local default = if field.default_value != null then ' DEFAULT ' + field.default_value else '';

      up_sql: 'ALTER TABLE %s ADD COLUMN %s %s%s%s;' % [table_name, field.name, typeStr, nullable, default],
      down_sql: 'ALTER TABLE %s DROP COLUMN %s;' % [table_name, field.name],
    },
  },

  // クエリヘルパー
  query: {
    select(table, fields='*', conditions=null, order_by=null, limit=null):: {
      sql: std.join(' ', [
        'SELECT',
        if std.isArray(fields) then std.join(', ', fields) else fields,
        'FROM',
        table,
      ] + if conditions != null then ['WHERE', conditions] else [] +
         if order_by != null then ['ORDER BY', order_by] else [] +
         if limit != null then ['LIMIT', std.toString(limit)] else []),
    },

    insert(table, data):: {
      local columns = std.join(', ', std.objectFields(data)),
           values = std.join(', ', ['$' + std.toString(i+1) for i in std.range(0, std.length(std.objectFields(data))-1)]);
      sql: 'INSERT INTO %s (%s) VALUES (%s)' % [table, columns, values],
      params: [data[k] for k in std.objectFields(data)],
    },

    update(table, data, conditions):: {
      local sets = std.join(', ', ['%s = $%d' % [k, i+1] for i in std.range(0, std.length(std.objectFields(data))-1) for k in std.objectFields(data)]);
      sql: 'UPDATE %s SET %s WHERE %s' % [table, sets, conditions],
      params: [data[k] for k in std.objectFields(data)],
    },

    delete(table, conditions):: {
      sql: 'DELETE FROM %s WHERE %s' % [table, conditions],
    },
  },
}
