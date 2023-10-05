import App from './App.svelte';
// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore
import wasm from '../../rust/Cargo.toml';
import 'bootstrap/dist/css/bootstrap.min.css';

// import LogRocket from 'logrocket';
// LogRocket.init('vl1t2j/kolor');

const init = async () => {
  const bindings = await wasm();

  new App({
    target: document.body,
    props: {
      bindings,
    },
  });
};

init();
