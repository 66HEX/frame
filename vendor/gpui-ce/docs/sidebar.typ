#let sidebar(
  nav: (),
  current: none,
  title: "",
  home-url: "/",
  doc,
) = context {
  if target() != "html" {
    return doc
  }

  html.input(type: "checkbox", id: "t")
  html.label("t")
  html.div(id: "l", {
    html.nav({
      html.header(
        html.a(href: home-url, html.h1(title)),
      )
      html.ul(
        for section in nav {
          html.li(
            html.span(section.title),
          )
          for item in section.at("items", default: ()) {
            html.li(
              if item.id == current {
                html.a(href: item.url, aria-current: "page", item.title)
              } else {
                html.a(href: item.url, item.title)
              },
            )
          }
        },
      )
    })
    html.main(doc)
  })
  html.label("t")
}
