import React, { FC, useState } from 'react';

interface FooterProps {
  children: string;
  className: string;
}

interface MessageProps {
  children: string;
}

interface AppProps {
  className?: string;
}

interface HeaderProps {
  className: string;
}

interface TitleProps {
  children: string;
}

interface MainProps {
  className: string;
}

const FooterDefaultProps: Partial<FooterProps> = {
  children: "© 2024 Kotoba",
  className: "footer",
};

const MessageDefaultProps: Partial<MessageProps> = {
  children: "This component was generated from pure Jsonnet!",
};

const AppDefaultProps: Partial<AppProps> = {
  className: "app",
};

const HeaderDefaultProps: Partial<HeaderProps> = {
  className: "header",
};

const TitleDefaultProps: Partial<TitleProps> = {
  children: "Hello from Kotoba!",
};

const MainDefaultProps: Partial<MainProps> = {
  className: "main",
};

const Footer: FC<FooterProps> = (props) => {
  return (
    <footer className={props.className}>
      {props.children}
    </footer>
  );
};

const Message: FC<MessageProps> = (props) => {
  return (
    <p>{props.children}</p>
  );
};

const Header: FC<HeaderProps> = (props) => {
  return (
    <header className={props.className}>
      <Title {...TitleDefaultProps} />
    </header>
  );
};

const Title: FC<TitleProps> = (props) => {
  return (
    <h1>{props.children}</h1>
  );
};

const Main: FC<MainProps> = (props) => {
  return (
    <main className={props.className}>
      <Message {...MessageDefaultProps} />
    </main>
  );
};

const App: FC<AppProps> = (props) => {
  const [message, setMessage] = useState("Hello World!");
  const [count, setCount] = useState(0);

  return (
    <div className={props.className || "app"}>
      <Header {...HeaderDefaultProps} />
      <Main {...MainDefaultProps} />

      {/* Interactive section */}
      <section className="interactive">
        <h2>Interactive Demo</h2>
        <p>Count: {count}</p>
        <p>Message: {message}</p>
        <button onClick={() => setCount(count + 1)}>Increment</button>
        <button onClick={() => setCount(0)}>Reset Count</button>
        <button onClick={() => setMessage("Updated from Kotoba!")}>Update Message</button>
      </section>

      <Footer {...FooterDefaultProps} />
    </div>
  );
};

export default App;