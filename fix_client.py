#!/usr/bin/env python3
"""Remove all remaining interceptor calls from client.rs"""

import re

def fix_client():
    with open('src/client.rs', 'r') as f:
        content = f.read()

    # Remove call_before_request calls (multi-line)
    content = re.sub(
        r'        self\.call_before_request\([^)]+\)\s*\.await\?;\n',
        '',
        content,
        flags=re.DOTALL
    )

    # Replace handle_api_error with map_api_error (multi-line, in Err branch)
    content = re.sub(
        r'(\s+)self\.handle_api_error\(e,\s*[^)]+\)',
        r'\1map_api_error(e)',
        content,
        flags=re.DOTALL
    )

    # Remove call_after_response calls (multi-line)
    content = re.sub(
        r'        self\.call_after_response\([^)]+\)\s*\.await;\n',
        '',
        content,
        flags=re.DOTALL
    )

    # Remove any remaining references to metadata variable
    content = re.sub(r'&mut metadata|&metadata|\bmetadata\b,?\s*', '', content)

    with open('src/client.rs', 'w') as f:
        f.write(content)

    print("✅ Fixed client.rs")

if __name__ == '__main__':
    fix_client()
