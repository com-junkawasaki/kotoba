import { React } from 'react';
import { FC, ReactElement } from '@types/react';

const Header: FC<HeaderProps> = (props) => {
  return (
    <header className="app-header" title="Test Application" />  );
};


const App: FC<AppProps> = (props) => {
  return (
    <div className="app-container">
      <header className="app-header" title="Test Application" />      <main className="app-content" />    </div>  );
};


const Content: FC<ContentProps> = (props) => {
  return (
    <main className="app-content" />  );
};


const App: FC = () => {

  return (
    <div className="app-container">
      <header className="app-header" title="Test Application" />      <main className="app-content" />    </div>  );
};

export default App;

