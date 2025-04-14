# JSONB + DataFusion Integration Notes

## Challenges
- **JSONB Type Handling**:  
  - Postgres returns JSONB as a string using `doc::text`, which we parse with `serde_json`.  
  - DataFusion doesn’t natively support JSON types, so for now we treat JSONB as a `Utf8` column and use functions like `json_extract_path` to extract JSON data.  
  - **Future Work**: Develop native JSON support within DataFusion to improve type handling and query expressiveness.

- **Performance**:  
  - Initial tests on 1K documents showed a query time of roughly ~80ms.  
  - The prototype does not yet leverage GIN indexing for filtering, resulting in suboptimal performance.  
  - **Future Improvement**: Integrate GIN index support (planned in M1.2) to significantly enhance filtering speed on larger datasets.

- **Schema Mapping**:  
  - The current process requires manual mapping from Postgres schemas to DataFusion’s Arrow schema, which adds maintenance overhead.  
  - **Limitation**: This manual mapping impairs dynamic table support and complicates schema evolution.  
  - **Next Step**: Automate schema mapping to streamline integration and support dynamic changes.

- **Error Handling**:  
  - Errors from `tokio-postgres` are not well-mapped to DataFusion’s error types, which can result in cryptic messages for users.  
  - **Required Change**: Implement an error translation layer to convert low-level database errors into clear, user-friendly messages that integrate with DataFusion’s error model.

## Next Steps
- **Performance Optimization**:  
  - Implement GIN index support to enable efficient filtering on large sets of JSONB documents.
  
- **Enhanced JSON Support**:  
  - Develop native JSON path operators in DataFusion, improving query performance and capabilities compared to the current workaround.
  
- **Scalability Testing**:  
  - Benchmark queries with larger datasets (10K+ documents) to evaluate performance under scale and adjust optimizations accordingly.
  
- **Schema Automation**:  
  - Automate the mapping from Postgres schemas to DataFusion’s Arrow schema to support dynamic table structures and reduce manual overhead.
  
- **Improved Error Reporting**:  
  - Create a standardized error mapping layer that converts `tokio-postgres` errors into DataFusion errors, making troubleshooting easier for end users.

## Observations
- **Row Count**: Test queries returned approximately 1,003 rows.
- **Query Performance**: Measured query execution time around 5.55ms in some scenarios, demonstrating promise for further optimization with improved indexing and operator support.
