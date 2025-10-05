// KotobaScript REPL Application

class KotobaREPL {
    constructor() {
        this.editor = null;
        this.output = null;
        this.isRunning = false;
        this.autoRunEnabled = true;
        this.init();
    }

    init() {
        this.setupMonaco();
        this.setupEventListeners();
        this.loadExamples();
        this.setDefaultCode();
    }

    setupMonaco() {
        // Load Monaco Editor
        require.config({ paths: { vs: 'https://unpkg.com/monaco-editor@0.45.0/min/vs' } });

        require(['vs/editor/editor.main'], () => {
            // Create editor
            this.editor = monaco.editor.create(document.getElementById('editor'), {
                value: '',
                language: 'json',
                theme: 'vs-light',
                fontSize: 14,
                minimap: { enabled: false },
                scrollBeyondLastLine: false,
                automaticLayout: true,
                wordWrap: 'on',
                tabSize: 2,
                insertSpaces: true,
                renderWhitespace: 'selection',
                bracketPairColorization: { enabled: true },
                guides: {
                    bracketPairs: true,
                    indentation: true
                }
            });

            // Configure Jsonnet language support
            monaco.languages.setMonarchTokensProvider('json', {
                tokenizer: {
                    root: [
                        // Jsonnet-specific keywords
                        [/\b(local|function|import|importstr|assert|if|then|else|for|in)\b/, 'keyword'],
                        // Standard JSON tokens
                        [/"/, { token: 'string.quote', bracket: '@open', next: '@string' }],
                        [/[{}]/, '@brackets'],
                        [/[[\]]/, '@brackets'],
                        [/:/, 'delimiter'],
                        [/,/, 'delimiter'],
                        [/\d+/, 'number'],
                        [/\b(true|false|null)\b/, 'keyword'],
                        [/\bstd\./, 'type', '@stdlib'],
                        [/[a-zA-Z_]\w*/, 'identifier']
                    ],
                    string: [
                        [/[^\\"]+/, 'string'],
                        [/\\./, 'string.escape'],
                        [/"/, { token: 'string.quote', bracket: '@close', next: '@pop' }]
                    ],
                    stdlib: [
                        [/\w+/, 'type'],
                        [/\./, 'delimiter', '@stdlib']
                    ]
                }
            });

            // Set up auto-run
            this.editor.onDidChangeModelContent(() => {
                if (this.autoRunEnabled && !this.isRunning) {
                    this.debounceRun();
                }
            });

            console.log('Monaco Editor initialized');
        });
    }

    setupEventListeners() {
        // Run button
        document.getElementById('run-btn').addEventListener('click', () => {
            this.runCode();
        });

        // Clear button
        document.getElementById('clear-btn').addEventListener('click', () => {
            this.clearOutput();
            this.editor.setValue('');
        });

        // Share button
        document.getElementById('share-btn').addEventListener('click', () => {
            this.shareCode();
        });

        // Format button
        document.getElementById('format-btn').addEventListener('click', () => {
            this.formatCode();
        });

        // Auto-run toggle
        document.getElementById('auto-run').addEventListener('change', (e) => {
            this.autoRunEnabled = e.target.checked;
        });

        // Examples button
        document.getElementById('examples-btn').addEventListener('click', () => {
            this.showExamples();
        });

        // Get output element
        this.output = document.getElementById('output');
    }

    loadExamples() {
        const examples = document.querySelectorAll('.example-item');
        examples.forEach(example => {
            example.addEventListener('click', () => {
                const code = example.dataset.code;
                this.editor.setValue(code);
                this.runCode();
            });
        });
    }

    setDefaultCode() {
        const defaultCode = `{
  // Welcome to KotobaScript REPL!
  greeting: "Hello, KotobaScript!",
  features: [
    "Declarative programming",
    "Jsonnet syntax",
    "Functional programming",
    "Type safety"
  ],
  version: "0.1.0",
  demo: {
    math: 2 + 3 * 4,
    string: "Hello " + "World!",
    array: [1, 2, 3, 4, 5],
    object: {
      nested: true,
      value: 42
    }
  }
}`;
        this.editor?.setValue(defaultCode);
    }

    debounceRun() {
        clearTimeout(this.runTimeout);
        this.runTimeout = setTimeout(() => {
            this.runCode();
        }, 500);
    }

    async runCode() {
        if (this.isRunning) return;

        this.isRunning = true;
        const runBtn = document.getElementById('run-btn');
        const originalText = runBtn.innerHTML;

        try {
            runBtn.innerHTML = '<div class="loading"></div> Running...';
            runBtn.disabled = true;

            const code = this.editor.getValue();
            const result = await this.evaluateCode(code);

            this.displayResult(result);

        } catch (error) {
            this.displayError(error);
        } finally {
            this.isRunning = false;
            runBtn.innerHTML = originalText;
            runBtn.disabled = false;
        }
    }

    async evaluateCode(code) {
        // For now, simulate evaluation with basic JSON parsing
        // TODO: Replace with actual KotobaScript/WASM evaluation

        try {
            // Basic syntax validation
            if (!code.trim()) {
                throw new Error('Please enter some code to run.');
            }

            // Try to parse as JSON first (for basic compatibility)
            const result = JSON.parse(code);

            // Simulate some Jsonnet features
            return this.simulateJsonnetEvaluation(result);

        } catch (jsonError) {
            // If not valid JSON, try basic Jsonnet simulation
            return this.simulateJsonnetEvaluation(code);
        }
    }

    simulateJsonnetEvaluation(input) {
        // Simulate basic Jsonnet evaluation for demo purposes
        if (typeof input === 'string') {
            // Handle basic expressions
            if (input.includes('std.join')) {
                return this.simulateStdJoin(input);
            }
            if (input.includes('function')) {
                return this.simulateFunction(input);
            }
            if (input.includes('local')) {
                return this.simulateLocal(input);
            }
            if (input.includes('+') || input.includes('*') || input.includes('-') || input.includes('/')) {
                return this.simulateArithmetic(input);
            }

            // Default: treat as string
            return input;
        }

        // For objects and arrays, return as formatted JSON
        return input;
    }

    simulateStdJoin(code) {
        // Simple simulation of std.join
        const match = code.match(/std\.join\("([^"]+)",\s*\[([^\]]+)\]\)/);
        if (match) {
            const separator = match[1];
            const items = match[2].split(',').map(item => item.trim().replace(/"/g, ''));
            return items.join(separator);
        }
        return "Error: Invalid std.join syntax";
    }

    simulateFunction(code) {
        // Simple function simulation
        const match = code.match(/local\s+(\w+)\s*=\s*function\((\w+)\)\s*(.+?);\s*\1\((.+?)\)/);
        if (match) {
            const funcName = match[1];
            const param = match[2];
            const body = match[3];
            const arg = match[4];

            if (body.includes(param + ' * 2')) {
                return parseInt(arg) * 2;
            }
        }
        return "Function evaluation not fully implemented yet";
    }

    simulateLocal(code) {
        // Simple local variable simulation
        const match = code.match(/local\s+(\w+)\s*=\s*(.+?);\s*local\s+(\w+)\s*=\s*(.+?);\s*\1\s*\+\s*\3/);
        if (match) {
            const val1 = parseInt(match[2]);
            const val2 = parseInt(match[4]);
            return val1 + val2;
        }
        return "Local variable evaluation not fully implemented yet";
    }

    simulateArithmetic(code) {
        // Simple arithmetic simulation
        try {
            // Replace Jsonnet operators with JavaScript equivalents
            let jsCode = code
                .replace(/\b(\w+)\s*\+\s*(\w+)\b/g, '$1 + $2')  // Basic addition
                .replace(/(\d+)\s*\*\s*(\d+)/g, '$1 * $2')        // Multiplication
                .replace(/(\d+)\s*\+\s*(\d+)/g, '$1 + $2')        // Addition
                .replace(/(\d+)\s*-\s*(\d+)/g, '$1 - $2');        // Subtraction

            return eval(jsCode);
        } catch (e) {
            return "Arithmetic evaluation failed";
        }
    }

    displayResult(result) {
        this.output.className = 'output success';

        if (typeof result === 'object') {
            this.output.textContent = JSON.stringify(result, null, 2);
        } else {
            this.output.textContent = String(result);
        }
    }

    displayError(error) {
        this.output.className = 'output error';
        this.output.textContent = `Error: ${error.message || error}`;
    }

    clearOutput() {
        this.output.textContent = '';
        this.output.className = 'output';
    }

    formatCode() {
        if (!this.editor) return;

        const code = this.editor.getValue();
        try {
            // Try to format as JSON first
            const parsed = JSON.parse(code);
            const formatted = JSON.stringify(parsed, null, 2);
            this.editor.setValue(formatted);
        } catch (e) {
            // If not valid JSON, just trim whitespace
            const trimmed = code.trim();
            this.editor.setValue(trimmed);
        }
    }

    shareCode() {
        const code = this.editor.getValue();
        const encoded = encodeURIComponent(code);
        const url = `${window.location.origin}${window.location.pathname}?code=${encoded}`;

        // Copy to clipboard if supported
        if (navigator.clipboard) {
            navigator.clipboard.writeText(url).then(() => {
                this.showNotification('Share URL copied to clipboard!');
            }).catch(() => {
                this.showShareDialog(url);
            });
        } else {
            this.showShareDialog(url);
        }
    }

    showShareDialog(url) {
        const dialog = document.createElement('div');
        dialog.style.cssText = `
            position: fixed;
            top: 50%;
            left: 50%;
            transform: translate(-50%, -50%);
            background: white;
            padding: 2rem;
            border-radius: 8px;
            box-shadow: 0 10px 25px rgba(0,0,0,0.2);
            z-index: 1000;
            max-width: 500px;
            width: 90%;
        `;

        dialog.innerHTML = `
            <h3 style="margin-bottom: 1rem; color: var(--text-primary);">Share this code</h3>
            <input type="text" value="${url}" readonly style="width: 100%; padding: 0.5rem; border: 1px solid var(--border-color); border-radius: 4px; margin-bottom: 1rem;">
            <div style="text-align: right;">
                <button onclick="this.closest('div').remove()" style="padding: 0.5rem 1rem; background: var(--primary-color); color: white; border: none; border-radius: 4px; cursor: pointer;">Close</button>
            </div>
        `;

        document.body.appendChild(dialog);

        // Close on click outside
        dialog.addEventListener('click', (e) => {
            if (e.target === dialog) {
                dialog.remove();
            }
        });
    }

    showNotification(message) {
        const notification = document.createElement('div');
        notification.style.cssText = `
            position: fixed;
            top: 20px;
            right: 20px;
            background: var(--primary-color);
            color: white;
            padding: 1rem 1.5rem;
            border-radius: 8px;
            box-shadow: var(--shadow-lg);
            z-index: 1000;
            animation: slideIn 0.3s ease;
        `;

        notification.textContent = message;
        document.body.appendChild(notification);

        setTimeout(() => {
            notification.style.animation = 'slideOut 0.3s ease';
            setTimeout(() => notification.remove(), 300);
        }, 3000);
    }

    showExamples() {
        // Toggle examples dropdown
        const examplesList = document.querySelector('.examples-list');
        if (examplesList.style.display === 'none' || examplesList.style.display === '') {
            examplesList.style.display = 'grid';
        } else {
            examplesList.style.display = 'none';
        }
    }
}

// Initialize the REPL when the page loads
document.addEventListener('DOMContentLoaded', () => {
    window.kotobaREPL = new KotobaREPL();
});

// Add CSS animations
const style = document.createElement('style');
style.textContent = `
    @keyframes slideIn {
        from { transform: translateX(100%); opacity: 0; }
        to { transform: translateX(0); opacity: 1; }
    }

    @keyframes slideOut {
        from { transform: translateX(0); opacity: 1; }
        to { transform: translateX(100%); opacity: 0; }
    }
`;
document.head.appendChild(style);
