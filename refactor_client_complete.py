#!/usr/bin/env python3
"""
Complete client.rs refactoring - remove all interceptor code properly.
"""

import re

def refactor():
    with open('src/client.rs', 'r') as f:
        lines = f.readlines()

    output = []
    i = 0
    while i < len(lines):
        line = lines[i]

        # Skip interceptor imports
        if 'use crate::interceptor' in line:
            i += 1
            continue

        # Skip too_many_arguments allow
        if 'clippy::too_many_arguments' in line or (i > 0 and 'Allow this lint' in lines[i-1]):
            i += 1
            continue

        # Skip tokio::sync::RwLock
        if 'use tokio::sync::RwLock' in line:
            i += 1
            continue

        # Skip macro definition
        if '// Helper macro to generate' in line:
            # Skip until end of macro
            while i < len(lines) and not (lines[i].strip() == '}' and i + 1 < len(lines) and lines[i+1].strip() == '}'):
                i += 1
            i += 2  # Skip the closing braces
            continue

        # Skip macro invocations
        if 'impl_interceptor_helpers!' in line:
            i += 1
            continue

        # Update Client struct - remove interceptors field
        if '    interceptors: Arc<RwLock<InterceptorChain>>,' in line:
            i += 1
            continue

        # Update Debug impl
        if 'since InterceptorChain' in line:
            output.append('// Custom Debug implementation\n')
            i += 1
            continue

        if '.field("interceptors"' in line:
            i += 1
            continue

        # Remove interceptors initialization in new()
        if '            interceptors: Arc::new' in line:
            i += 1
            continue

        # Skip with_interceptor method
        if '    /// Add an interceptor' in line:
            # Skip until end of method
            while i < len(lines) and not (lines[i].strip() == '}' and (i + 1 >= len(lines) or not lines[i+1].strip().startswith('//'))):
                i += 1
            i += 1
            continue

        # Skip interceptors() method
        if 'pub(crate) fn interceptors' in line:
            # Skip until end of method
            while i < len(lines) and lines[i].strip() != '}':
                i += 1
            i += 1
            if i < len(lines) and lines[i].strip() == '}':
                i += 1
            continue

        # Skip "// Interceptor helper methods" block
        if '// Interceptor helper methods' in line:
            i += 1
            # Skip the impl Client block
            brace_count = 0
            while i < len(lines):
                if '{' in lines[i]:
                    brace_count += lines[i].count('{')
                if '}' in lines[i]:
                    brace_count -= lines[i].count('}')
                i += 1
                if brace_count == 0 and lines[i-1].strip() == '}':
                    break
            continue

        # Remove metadata HashMap declarations
        if '        let mut metadata = HashMap::new();' in line or '        let metadata = HashMap::new();' in line:
            i += 1
            continue

        # Remove call_before_request (handle multi-line)
        if '        self.call_before_request' in line:
            # Skip until we find .await?;
            while i < len(lines):
                if '.await?' in lines[i]:
                    i += 1
                    break
                i += 1
            continue

        # Replace handle_api_error with map_api_error
        if '.handle_api_error(' in line:
            # Collect the full statement
            stmt = line
            while i < len(lines) and not stmt.rstrip().endswith(';'):
                i += 1
                if i < len(lines):
                    stmt += lines[i]
            # Replace it
            output.append(f"{' ' * (len(line) - len(line.lstrip()))}map_api_error(e)\n")
            i += 1
            continue

        # Remove call_after_response (multi-line)
        if '        self.call_after_response' in line:
            # Skip until we find .await;
            while i < len(lines):
                if '.await;' in lines[i]:
                    i += 1
                    break
                i += 1
            continue

        # Keep everything else
        output.append(line)
        i += 1

    with open('src/client.rs', 'w') as f:
        f.writelines(output)

    print("✅ Client refactored")

if __name__ == '__main__':
    refactor()
