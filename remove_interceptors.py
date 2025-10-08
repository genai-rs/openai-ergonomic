#!/usr/bin/env python3
"""Remove all interceptor code from client.rs"""

import re

def remove_interceptors():
    with open('src/client.rs', 'r') as f:
        content = f.read()

    # 1. Remove interceptor imports
    content = re.sub(
        r'use crate::interceptor::\{[^}]+\};\n',
        '',
        content
    )

    # 2. Remove the lint allow for too_many_arguments
    content = re.sub(
        r'// Allow this lint.*?\n#!\[allow\(clippy::too_many_arguments\)\]\n\n',
        '',
        content,
        flags=re.DOTALL
    )

    # 3. Remove tokio::sync::RwLock import
    content = re.sub(r'use tokio::sync::RwLock;\n', '', content)

    # 4. Remove impl_interceptor_helpers macro definition
    content = re.sub(
        r'// Helper macro to generate interceptor helper methods.*?^}\n',
        '',
        content,
        flags=re.DOTALL | re.MULTILINE
    )

    # 5. Remove macro invocations
    content = re.sub(r'impl_interceptor_helpers!\([^)]+\);\n', '', content)

    # 6. Update Client struct
    content = re.sub(
        r'    interceptors: Arc<RwLock<InterceptorChain>>,',
        '',
        content
    )

    # 7. Update Debug impl
    content = re.sub(
        r'// Custom Debug implementation since InterceptorChain doesn\'t implement Debug',
        '// Custom Debug implementation',
        content
    )

    content = re.sub(
        r'            \.field\("interceptors", &"<InterceptorChain>"\)\n',
        '',
        content
    )

    # 8. Update Client::new() initialization
    content = re.sub(
        r'            interceptors: Arc::new\(RwLock::new\(InterceptorChain::new\(\)\)\),\n',
        '',
        content
    )

    # 9. Remove with_interceptor method
    content = re.sub(
        r'    /// Add an interceptor to the client\..*?^    \}\n',
        '',
        content,
        flags=re.DOTALL | re.MULTILINE
    )

    # 10. Remove interceptors() accessor
    content = re.sub(
        r'    /// Get a reference to the interceptor chain\..*?^    \}\n',
        '',
        content,
        flags=re.DOTALL | re.MULTILINE
    )

    # 11. Remove "// Interceptor helper methods" impl block
    content = re.sub(
        r'// Interceptor helper methods\nimpl Client \{.*?\n\}\n',
        '',
        content,
        flags=re.DOTALL
    )

    # 12. Remove metadata variable declarations
    content = re.sub(r'\n        let mut metadata = HashMap::new\(\);\n', '\n', content)

    # 13. Remove call_before_request calls
    content = re.sub(
        r'        self\.call_before_request\([^)]+\)\.await\?;\n',
        '',
        content
    )

    # 14. Replace handle_api_error with map_api_error
    content = re.sub(
        r'self\.handle_api_error\(e, [^)]+\)',
        'map_api_error(e)',
        content
    )

    # 15. Remove call_after_response calls
    content = re.sub(
        r'        self\.call_after_response\([^)]+\)\.await;\n',
        '',
        content
    )

    with open('src/client.rs', 'w') as f:
        f.write(content)

    print("✅ Removed all interceptor code from client.rs")

if __name__ == '__main__':
    remove_interceptors()
