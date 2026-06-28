#import "sidebar.typ": sidebar

#let template(current: str, doc) = {
  let site-nav = yaml("site-nav.yaml")

  show: sidebar.with(
    nav: site-nav,
    current: current,
    title: "GPUI-CE",
    home-url: "./index.html",
  )

  doc
}
