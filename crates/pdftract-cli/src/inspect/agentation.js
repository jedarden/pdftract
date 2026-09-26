// Mount Agentation on the standalone inspector pages.
import React from "react";
import { createRoot } from "react-dom/client";
import { Agentation } from "agentation";

const root = document.createElement("div");
root.id = "agentation-root";
document.body.appendChild(root);

createRoot(root).render(React.createElement(Agentation));
