import React, { FC } from 'react';
import { ReactElement } from '@types/react';

interface AppProps {
  className: string;
}


interface HeaderProps {
  title: string;
  className: string;
}


interface ContentProps {
  className: string;
}


const AppDefaultProps: Partial<AppProps> = {
  className: "app-container",
};


const HeaderDefaultProps: Partial<HeaderProps> = {
  title: "Test Application",
  className: "app-header",
};


const ContentDefaultProps: Partial<ContentProps> = {
  className: "app-content",
};


const Header: FC<HeaderProps> = (props) => {
  return (
    <header title="Test Application" className="app-header" />  );
};


const Content: FC<ContentProps> = (props) => {
  return (
    <main className="app-content" />  );
};


const App: FC = () => {

  return (
    <div className="app-container">
      <Header />
      <Content />
    </div>  );
};

export default App;

