/* eslint-disable */

if (import.meta.hot) {
  App.__hmrId__ = 'app';
  import.meta.hot.accept((newModule) => {
    window.__JINGE_HMR__.replace(newModule.App);
  });
}
