use crate::config::RouteConfig;
use crate::http::{HttpMethod, HttpRequest};
use std::path::Path;

pub struct Router {
    routes: Vec<RouteConfig>,
}

pub struct RouteMatch {
    pub route: RouteConfig,
    pub file_path: String,
    pub redirect_to: Option<String>,
}

pub enum RouteMatchResult {
    Match(RouteMatch),
    MethodNotAllowed(RouteMatch),
    NotFound,
}

impl Router {
    pub fn new(routes: Vec<RouteConfig>) -> Self {
        Router { routes }
    }

    pub fn match_route(&self, request: &HttpRequest) -> RouteMatchResult {
        for route in &self.routes {
            if request.path.starts_with(&route.path) {
                // Vérifier les redirections d'abord
                if let Some(redirect_to) = self.check_redirections(route, &request.path) {
                    return RouteMatchResult::Match(RouteMatch {
                        route: route.clone(),
                        file_path: String::new(),
                        redirect_to: Some(redirect_to),
                    });
                }
    
                let method_str = match request.method {
                    HttpMethod::GET => "GET",
                    HttpMethod::POST => "POST",
                    HttpMethod::DELETE => "DELETE",
                    HttpMethod::UNSUPPORTED => continue,
                };
    
                if route.methods.contains(&method_str.to_string()) {
                    let relative_path = request.path.trim_start_matches(&route.path);
                    let file_path = Path::new(&route.root).join(relative_path);
    
                    // Pour POST et DELETE, pas besoin de vérifier si le fichier existe
                    if matches!(request.method, HttpMethod::POST | HttpMethod::DELETE) {
                        return RouteMatchResult::Match(RouteMatch {
                            route: route.clone(),
                            file_path: file_path.to_string_lossy().into_owned(),
                            redirect_to: None,
                        });
                    }
    
                    // Pour GET, on vérifie si c'est un répertoire et on utilise le fichier index si spécifié
                    if file_path.is_dir() {
                        if let Some(ref index) = route.index {
                            let index_path = file_path.join(index);
                            if index_path.exists() {
                                return RouteMatchResult::Match(RouteMatch {
                                    route: route.clone(),
                                    file_path: index_path.to_string_lossy().into_owned(),
                                    redirect_to: None,
                                });
                            }
                        }
    
                        // Gérer le listing des répertoires si activé
                        if route.directory_listing {
                            return RouteMatchResult::Match(RouteMatch {
                                route: route.clone(),
                                file_path: file_path.to_string_lossy().into_owned(),
                                redirect_to: None,
                            });
                        }
                    } else if file_path.exists() {
                        return RouteMatchResult::Match(RouteMatch {
                            route: route.clone(),
                            file_path: file_path.to_string_lossy().into_owned(),
                            redirect_to: None,
                        });
                    }
                } else {
                    // La route existe mais la méthode n'est pas autorisée
                    return RouteMatchResult::MethodNotAllowed(RouteMatch {
                        route: route.clone(),
                        file_path: String::new(),
                        redirect_to: None,
                    });
                }
            }
        }
        RouteMatchResult::NotFound
    }
    fn check_redirections(&self, route: &RouteConfig, path: &str) -> Option<String> {
        for (from, to) in &route.redirections {
            if path == from {
                return Some(to.clone());
            }
        }
        None
    }
}