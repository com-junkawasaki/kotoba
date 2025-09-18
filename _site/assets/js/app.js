// Simple React App generated from Kotoba
const { useState } = React;

const App = () => {
    const [count, setCount] = useState(0);

    return React.createElement('div', { className: 'app' },
        React.createElement('div', { className: 'hero' },
            React.createElement('h1', null, '🚀 Kotoba React Demo'),
            React.createElement('p', null, 'Built with pure Jsonnet and kotoba2tsx!'),
            React.createElement('p', null, `Count: ${count}`),
            React.createElement('br'),
            React.createElement('button', {
                className: 'btn',
                onClick: () => setCount(count + 1)
            }, 'Increment'),
            React.createElement('br'),
            React.createElement('br'),
            React.createElement('button', {
                className: 'btn',
                onClick: () => setCount(0)
            }, 'Reset')
        )
    );
};

const root = ReactDOM.createRoot(document.getElementById('root'));
root.render(React.createElement(App));