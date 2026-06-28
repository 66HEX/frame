#let collapsible(summary: [], open: false, name: "", content) = context {
  if target() != "html" {
    summary
    content
    return
  }

  html.details(
    name: name,
    open: open,
    {
      html.summary(summary)
      content
    },
  )
}
