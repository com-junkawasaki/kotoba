import React, { FC } from 'react';

interface HeaderProps {
  title: string;
}

interface ButtonProps {
  text: string;
  onClick?: () => void;
}

const Header: FC<HeaderProps> = ({ title }) => {
  return (
    <header>
      <h1>{title}</h1>
    </header>
  );
};

const Button: FC<ButtonProps> = ({ text, onClick }) => {
  return (
    <button onClick={onClick}>
      {text}
    </button>
  );
};

const App: FC = () => {
  return (
    <div>
      <Header title="Kotoba App" />
      <Button text="Click me" onClick={() => alert('Hello from Kotoba!')} />
    </div>
  );
};

export default App;

