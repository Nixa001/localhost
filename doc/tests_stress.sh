#!/bin/bash

echo "Starting stress tests..."

# Test 1: Basic stress test
siege -b -c100 -t30S http://localhost:8080

# Test 2: High concurrency
siege -b -c500 -t30S http://localhost:8080

# Test 3: Long duration
siege -b -c50 -t5M http://localhost:8080

echo "Stress tests completed."